import { useEffect, useRef } from "react";
import * as THREE from "three";
import { OrbitControls } from "three/addons/controls/OrbitControls.js";

import type { BoundsMm } from "../step-import/contracts";
import type { PreviewMeshResponse } from "./contracts";

interface Props {
  preview: PreviewMeshResponse;
  bounds: BoundsMm;
  selectedComponentId: string | null;
  onSelect: (componentId: string) => void;
}

export function MeshCanvas({ preview, bounds, selectedComponentId, onSelect }: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  const meshesRef = useRef<Map<string, THREE.Mesh>>(new Map());

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    const scene = new THREE.Scene();
    scene.background = new THREE.Color("#12171c");
    const center = new THREE.Vector3(...bounds.min_xyz).add(new THREE.Vector3(...bounds.max_xyz)).multiplyScalar(0.5);
    const diagonal = new THREE.Vector3(...bounds.max_xyz).sub(new THREE.Vector3(...bounds.min_xyz)).length();
    const radius = Math.max(diagonal, 1);
    const camera = new THREE.PerspectiveCamera(45, 1, Math.max(radius / 10000, 0.001), radius * 100);
    camera.up.set(0, 0, 1);
    camera.position.copy(center).add(new THREE.Vector3(radius, -radius, radius * 0.75));
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
    const grid = new THREE.GridHelper(radius * 2, 10, 0x355063, 0x253541);
    grid.rotation.x = Math.PI / 2;
    grid.position.z = bounds.min_xyz[2];
    scene.add(grid);
    const axes = new THREE.AxesHelper(radius * 0.2);
    axes.position.copy(center);
    scene.add(axes);

    const meshes = new Map<string, THREE.Mesh>();
    for (const component of preview.components) {
      const geometry = new THREE.BufferGeometry();
      geometry.setAttribute("position", new THREE.Float32BufferAttribute(component.positions.flat(), 3));
      geometry.setAttribute("normal", new THREE.Float32BufferAttribute(component.normals.flat(), 3));
      geometry.setIndex(component.indices);
      const color = component.color ? new THREE.Color(`rgb(${component.color.join(",")})`) : new THREE.Color("#78a9ca");
      const material = new THREE.MeshStandardMaterial({ color, side: THREE.DoubleSide, roughness: 0.8 });
      const mesh = new THREE.Mesh(geometry, material);
      mesh.userData.componentId = component.component_id;
      scene.add(mesh);
      meshes.set(component.component_id, mesh);
    }
    meshesRef.current = meshes;

    const resize = () => {
      const width = container.clientWidth;
      const height = container.clientHeight;
      if (width <= 0 || height <= 0) return;
      camera.aspect = width / height;
      camera.updateProjectionMatrix();
      renderer.setSize(width, height);
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
    let frame = 0;
    const draw = () => {
      frame = requestAnimationFrame(draw);
      controls.update();
      renderer.render(scene, camera);
    };
    draw();

    return () => {
      cancelAnimationFrame(frame);
      renderer.domElement.removeEventListener("click", pick);
      observer.disconnect();
      controls.dispose();
      for (const mesh of meshes.values()) {
        mesh.geometry.dispose();
        (mesh.material as THREE.Material).dispose();
      }
      meshesRef.current = new Map();
      renderer.dispose();
      container.removeChild(renderer.domElement);
    };
  }, [preview, bounds, onSelect]);

  useEffect(() => {
    for (const [componentId, mesh] of meshesRef.current) {
      const material = mesh.material as THREE.MeshStandardMaterial;
      material.emissive.setHex(componentId === selectedComponentId ? 0x254e67 : 0x000000);
    }
  }, [selectedComponentId, preview]);

  return <div ref={containerRef} className="preview-canvas" aria-label="3D model preview" />;
}
