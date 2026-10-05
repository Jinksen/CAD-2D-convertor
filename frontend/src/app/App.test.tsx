import { fireEvent, render, screen } from "@testing-library/react";
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

  it("opens the file chooser from the toolbar and names the plane controls", () => {
    render(<App />);
    const fileInput = screen.getByLabelText(/choose step file/i);
    let opened = false;
    fileInput.addEventListener("click", () => { opened = true; });
    fireEvent.click(screen.getByRole("button", { name: /open step/i }));
    expect(opened).toBe(true);
    expect(screen.getByRole("group", { name: /section plane/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "XY" })).toBeDisabled();
  });
});
