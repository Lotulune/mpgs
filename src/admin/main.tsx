import React from "react";
import ReactDOM from "react-dom/client";
import { AdminApp } from "./AdminApp";

ReactDOM.createRoot(document.getElementById("admin-root") as HTMLElement).render(
  <React.StrictMode>
    <AdminApp />
  </React.StrictMode>,
);
