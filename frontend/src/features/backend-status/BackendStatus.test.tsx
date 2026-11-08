import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { BackendStatus } from "./BackendStatus";

describe("BackendStatus", () => {
  it("announces an online backend", () => {
    render(
      <BackendStatus
        health={{ state: "online", service: "cad2maxwell-backend", apiVersion: "v1" }}
      />,
    );

    expect(screen.getByRole("status")).toHaveTextContent(/backend online/i);
  });

  it("announces an offline backend", () => {
    render(<BackendStatus health={{ state: "offline" }} />);

    expect(screen.getByRole("status")).toHaveTextContent(/backend offline/i);
  });

  it("reports the imported body count and source units", () => {
    render(<BackendStatus health={{ state: "online", service: "cad2maxwell-backend", apiVersion: "v1" }}
      bodyCount={2} sourceUnit="inch" />);
    expect(screen.getByRole("status")).toHaveTextContent("Bodies: 2");
    expect(screen.getByRole("status")).toHaveTextContent("Source: inch");
  });

  it("counts section errors and warnings in sentence case, or reports that checks passed", () => {
    const health = { state: "online", service: "cad2maxwell-backend", apiVersion: "v1" } as const;
    const { rerender } = render(<BackendStatus health={health} sectionStats={{ regions: 3, errors: 2, warnings: 1 }} />);
    expect(screen.getByText("2 errors")).toHaveClass("error");
    expect(screen.getByText("1 warning")).toHaveClass("warning");
    rerender(<BackendStatus health={health} sectionStats={{ regions: 3, errors: 0, warnings: 0 }} />);
    expect(screen.getByRole("status")).toHaveTextContent("Checks passed");
  });
});
