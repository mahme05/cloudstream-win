// src/main.tsx
// Entry point for the React app.
// React renders into the <div id="root"> in index.html.

import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
