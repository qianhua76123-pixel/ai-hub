import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { BrowserRouter } from "react-router-dom";
import Tasks from "./Tasks";

describe("Tasks", () => {
  it("renders without crashing", () => {
    render(<BrowserRouter><Tasks /></BrowserRouter>);
    expect(screen.getByText("任务中心")).toBeInTheDocument();
  });

  it("shows new task button", () => {
    render(<BrowserRouter><Tasks /></BrowserRouter>);
    expect(screen.getByText("新建任务")).toBeInTheDocument();
  });
});
