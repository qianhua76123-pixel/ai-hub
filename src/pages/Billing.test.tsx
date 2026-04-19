import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { BrowserRouter } from "react-router-dom";
import Billing from "./Billing";

describe("Billing", () => {
  it("renders without crashing", () => {
    render(<BrowserRouter><Billing /></BrowserRouter>);
    expect(screen.getByText("订阅管理")).toBeInTheDocument();
  });

  it("shows all tabs", () => {
    render(<BrowserRouter><Billing /></BrowserRouter>);
    expect(screen.getByText("订阅 vs API")).toBeInTheDocument();
    expect(screen.getByText("模型价格")).toBeInTheDocument();
    expect(screen.getByText("模型评比")).toBeInTheDocument();
    expect(screen.getByText("订阅计划")).toBeInTheDocument();
    expect(screen.getByText("ROI 分析")).toBeInTheDocument();
  });
});
