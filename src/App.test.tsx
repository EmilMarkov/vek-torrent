import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import App from "./App";

describe("App", () => {
  it("рендерит заголовок приложения", () => {
    render(<App />);
    expect(screen.getByText("VEK Torrent")).toBeInTheDocument();
  });
});
