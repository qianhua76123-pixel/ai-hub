import React from "react";
import ReactDOM from "react-dom/client";
import { BrowserRouter, Routes, Route } from "react-router-dom";
import Layout from "./components/Layout";
import Dashboard from "./pages/Dashboard";
import Providers from "./pages/Providers";
import Tasks from "./pages/Tasks";
import Usage from "./pages/Usage";
import Billing from "./pages/Billing";
import SettingsPage from "./pages/SettingsPage";
import Conversations from "./pages/Conversations";
import Advisor from "./pages/Advisor";
import News from "./pages/News";
import Rankings from "./pages/Rankings";
import "./index.css";
import { initTheme } from "./lib/theme";
initTheme();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <BrowserRouter>
      <Routes>
        <Route element={<Layout />}>
          <Route path="/" element={<Dashboard />} />
          <Route path="/providers" element={<Providers />} />
          <Route path="/tasks" element={<Tasks />} />
          <Route path="/conversations" element={<Conversations />} />
          <Route path="/rankings" element={<Rankings />} />
          <Route path="/usage" element={<Usage />} />
          <Route path="/billing" element={<Billing />} />
          <Route path="/advisor" element={<Advisor />} />
          <Route path="/news" element={<News />} />
          <Route path="/settings" element={<SettingsPage />} />
        </Route>
      </Routes>
    </BrowserRouter>
  </React.StrictMode>
);
