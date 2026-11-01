import { useEffect, useRef } from "react";
import * as THREE from "three";
import { OrbitControls } from "three/addons/controls/OrbitControls.js";

import type { BoundsMm } from "../step-import/contracts";
import type { PreviewMeshResponse } from "./contracts";
import type { SectionPlaneKind, Vec3 } from "../section/plane";
import { cameraPosition, VIEW_DIRECTIONS, type StandardView } from "./cameraViews";
import { createRenderScheduler, type RenderScheduler } from "./renderScheduler";
import { planeGuideFrame } from "./sectionPlane";

const VIEW_BUTTONS: { view: StandardView | "fit"; label: string; title: string }[] = [
  { view: "fit", label: "Fit", title: "Fit all bodies in view" },
  { view: "iso", label: "Iso", title: "Isometric view" },
  { view: "top", label: "Top", title: "Top view (look along -Z)" },
  { view: "front", label: "Front", title: "Front view (look along +Y)" },
  { view: "right", label: "Right", title: "Right view (look along -X)" },
];

interface Props {
  preview: PreviewMeshResponse;
  bounds: BoundsMm;
  selectedComponentId: string | null;
  onSelect: (componentId: string) => void;
  plane: SectionPlaneKind;
  offsetMm: number;
  normal: Vec3;
}

export function MeshCanvas({ preview, bounds, selectedComponentId, onSelect, plane, offsetMm, normal }: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  const meshesRef = useRef<Map<string, THREE.Mesh>>(new Map());
  const guideRef = useRef<THREE.Group | null>(null);
  const schedulerRef = useRef<RenderScheduler | null>(null);
  const applyViewRef = useRef<((view: StandardView | "fit") => void) | null>(null);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    const scene = new THREE.Scene();
    scene.background = new THREE.Color(getComputedStyle(container).getPropertyValue("--canvas-bg").trim() || "#ffffff");
    const center = new THREE.Vector3(...bounds.min_xyz).add(new THREE.Vector3(...bounds.max_xyz)).multiplyScalar(0.5);
    const diagonal = new THREE.Vector3(...bounds.max_xyz).sub(new THREE.Vector3(...bounds.min_xyz)).length();
    const radius = Math.max(diagonal, 1);
    const camera = new THREE.PerspectiveCamera(45, 1, Math.max(radius / 10000, 0.001), radius * 100);
    camera.up.set(0, 0, 1);
    camera.position.set(...cameraPosition(center.toArray(), VIEW_DIRECTIONS.iso, radius / 2, camera.fov, 1));
    camera.lookAt(center);
    let renderer: THREE.WebGLRenderer;
    try {
      renderer = new THREE.WebGLRenderer({ antialias: true });
    } catch {
      throw new Error("3D graphics are unavailable. Check WebView2 and your graphics driver.");
    }
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    container.appendChild(renderer.domElement);
    const controls = new OrbitControls(camera, renderer.domElement);
    controls.target.copy(center);
    controls.enableDamping = true;
    controls.update();
    scene.add(new THREE.AmbientLight(0xffffff, 1.7));
    const light = new THREE.DirectionalLight(0xffffff, 2);
    light.position.copy(center).add(new THREE.Vector3(radius, -radius, radius * 2));
    scene.add(light);
    const grid = new THREE.GridHelper(radius * 2, 10, 0xb9bec4, 0xdfe2e5);
    grid.rotation.x = Math.PI / 2;
    grid.position.z = bounds.min_xyz[2];
    scene.add(grid);
    const axes = new THREE.AxesHelper(radius * 0.2);
    axes.position.copy(center);
    scene.add(axes);

    const guide = new THREE.Group();
    const guideGeometry = new THREE.PlaneGeometry(1, 1);
    const guideSurface = new THREE.Mesh(guideGeometry, new THREE.MeshBasicMaterial({
      color: 0x4bb3fd, side: THREE.DoubleSide, transparent: true, opacity: 0.2,
      depthTest: false, depthWrite: false,
    }));
    const guideEdges = new THREE.LineSegments(
      new THREE.EdgesGeometry(guideGeometry),
      new THREE.LineBasicMaterial({ color: 0x1a5fb4, depthTest: false }),
    );
    guide.add(guideSurface, guideEdges);
    scene.add(guide);
    guideRef.current = guide;

    const meshes = new Map<string, THREE.Mesh>();
    const edgeLines: THREE.LineSegments[] = [];
    const edgeMaterial = new THREE.LineBasicMaterial({ color: 0x1f2328 });
    for (const component of preview.components) {
      const geometry = new THREE.BufferGeometry();
      const position = new THREE.BufferAttribute(component.positions, 3);
      geometry.setAttribute("position", position);
      geometry.setAttribute("normal", new THREE.BufferAttribute(component.normals, 3));
      geometry.setIndex(new THREE.BufferAttribute(component.indices, 1));
      const color = component.color ? new THREE.Color(`rgb(${component.color.join(",")})`) : new THREE.Color("#78a9ca");
      // Polygon offset pushes faces back so the coincident feature edges stay visible.
      const material = new THREE.MeshStandardMaterial({
        color, side: THREE.DoubleSide, roughness: 0.8,
        polygonOffset: true, polygonOffsetFactor: 1, polygonOffsetUnits: 1,
      });
      const mesh = new THREE.Mesh(geometry, material);
      mesh.userData.componentId = component.component_id;
      scene.add(mesh);
      meshes.set(component.component_id, mesh);
      if (component.edgeIndices.length > 0) {
        const edgeGeometry = new THREE.BufferGeometry();
        edgeGeometry.setAttribute("position", position);
        edgeGeometry.setIndex(new THREE.BufferAttribute(component.edgeIndices, 1));
        const lines = new THREE.LineSegments(edgeGeometry, edgeMaterial);
        lines.raycast = () => undefined; // picking targets bodies only
        scene.add(lines);
        edgeLines.push(lines);
      }
    }
    meshesRef.current = meshes;

    const scheduler = createRenderScheduler(
      () => { controls.update(); renderer.render(scene, camera); },
      () => container.clientWidth > 0 && container.clientHeight > 0,
    );
    schedulerRef.current = scheduler;
    // OrbitControls emits "change" on input and on every damping step until it settles.
    controls.addEventListener("change", scheduler.request);
    applyViewRef.current = (view) => {
      const direction: Vec3 = view === "fit"
        ? camera.position.clone().sub(controls.target).toArray()
        : VIEW_DIRECTIONS[view];
      controls.target.copy(center);
      camera.position.set(...cameraPosition(center.toArray(), direction, radius / 2, camera.fov, camera.aspect));
      controls.update();
      scheduler.request();
    };

    const resize = () => {
      const width = container.clientWidth;
      const height = container.clientHeight;
      if (width <= 0 || height <= 0) return;
      camera.aspect = width / height;
      camera.updateProjectionMatrix();
      renderer.setSize(width, height);
      scheduler.request();
    };
    const observer = new ResizeObserver(resize);
    observer.observe(container);
    resize();

    const raycaster = new THREE.Raycaster();
    const pointer = new THREE.Vector2();
    const pick = (event: MouseEvent) => {
      const rect = renderer.domElement.getBoundingClientRect();
      if (!rect.width || !rect.height) return;
      pointer.set(((event.clientX - rect.left) / rect.width) * 2 - 1,
        -((event.clientY - rect.top) / rect.height) * 2 + 1);
      raycaster.setFromCamera(pointer, camera);
      const hits = raycaster.intersectObjects([...meshes.values()]);
      if (hits.length === 0) return;
      const componentId: unknown = hits[0].object.userData.componentId;
      if (typeof componentId === "string") onSelect(componentId);
    };
    renderer.domElement.addEventListener("click", pick);
    scheduler.request();

    return () => {
      scheduler.dispose();
      schedulerRef.current = null;
      applyViewRef.current = null;
      controls.removeEventListener("change", scheduler.request);
      renderer.domElement.removeEventListener("click", pick);
      observer.disconnect();
      controls.dispose();
      for (const mesh of meshes.values()) {
        mesh.geometry.dispose();
        (mesh.material as THREE.Material).dispose();
      }
      for (const lines of edgeLines) lines.geometry.dispose();
      edgeMaterial.dispose();
      meshesRef.current = new Map();
      guideGeometry.dispose();
      guideSurface.material.dispose();
      guideEdges.geometry.dispose();
      guideEdges.material.dispose();
      guideRef.current = null;
      renderer.dispose();
      container.removeChild(renderer.domElement);
    };
  }, [preview, bounds, onSelect]);

  useEffect(() => {
    for (const [componentId, mesh] of meshesRef.current) {
      const material = mesh.material as THREE.MeshStandardMaterial;
      material.emissive.setHex(componentId === selectedComponentId ? 0x1a3f6b : 0x000000);
    }
    schedulerRef.current?.request();
  }, [selectedComponentId, preview]);

  useEffect(() => {
    const guide = guideRef.current;
    if (!guide) return;
    const frame = planeGuideFrame(bounds, plane, offsetMm, normal);
    guide.position.set(...frame.center);
    guide.rotation.set(...frame.rotation);
    guide.scale.set(frame.width, frame.height, 1);
    schedulerRef.current?.request();
  }, [bounds, plane, offsetMm, normal, preview]);

  return <>
    <div className="preview-toolbar" role="toolbar" aria-label="3D view">
      {VIEW_BUTTONS.map(({ view, label, title }) =>
        <button key={view} type="button" title={title} onClick={() => applyViewRef.current?.(view)}>{label}</button>)}
    </div>
    <div ref={containerRef} className="preview-canvas" aria-label="3D model preview" />
  </>;
}
