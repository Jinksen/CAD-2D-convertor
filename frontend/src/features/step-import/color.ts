export function rgb(color: [number, number, number] | null): string | undefined {
  return color ? `rgb(${color.map(String).join(",")})` : undefined;
}
