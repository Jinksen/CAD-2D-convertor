import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { App } from "./App";

describe("CAD2Maxwell workspace", () => {
  it("renders the engineering workspace regions", () => {
    render(<App />);

    expect(screen.getByRole("banner", { name: /application toolbar/i })).toBeInTheDocument();
    expect(screen.getByRole("navigation", { name: /model tree/i })).toBeInTheDocument();
    expect(screen.getByRole("main", { name: /viewport/i })).toBeInTheDocument();
    expect(screen.getByRole("complementary", { name: /properties/i })).toBeInTheDocument();
    expect(screen.getByRole("status")).toBeInTheDocument();
  });
});
