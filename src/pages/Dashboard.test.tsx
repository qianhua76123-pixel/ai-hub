import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { BrowserRouter } from "react-router-dom";
import Dashboard from "./Dashboard";

function renderWithRouter(ui: React.ReactElement) {
  return render(<BrowserRouter>{ui}</BrowserRouter>);
}

describe("Dashboard", () => {
  it("renders without crashing", () => {
    renderWithRouter(<Dashboard />);
    expect(screen.getByText("总览")).toBeInTheDocument();
  });

  it("shows stat cards", () => {
    renderWithRouter(<Dashboard />);
    expect(screen.getByText("已接入工具")).toBeInTheDocument();
    expect(screen.getByText("今日请求")).toBeInTheDocument();
    expect(screen.getByText("今日 Token")).toBeInTheDocument();
    expect(screen.getByText("今日费用")).toBeInTheDocument();
  });
});
