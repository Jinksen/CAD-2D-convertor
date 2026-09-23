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
});
