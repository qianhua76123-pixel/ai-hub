import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { BrowserRouter } from "react-router-dom";
import SettingsPage from "./SettingsPage";

describe("SettingsPage", () => {
  it("renders without crashing", () => {
    render(<BrowserRouter><SettingsPage /></BrowserRouter>);
    expect(screen.getByText("设置")).toBeInTheDocument();
  });

  it("shows theme section", () => {
    render(<BrowserRouter><SettingsPage /></BrowserRouter>);
    expect(screen.getByText("外观")).toBeInTheDocument();
    expect(screen.getByText("跟随系统")).toBeInTheDocument();
    expect(screen.getByText("浅色")).toBeInTheDocument();
    expect(screen.getByText("深色")).toBeInTheDocument();
  });

  it("shows proxy gateway section", () => {
    render(<BrowserRouter><SettingsPage /></BrowserRouter>);
    expect(screen.getByText("代理网关")).toBeInTheDocument();
  });
});
