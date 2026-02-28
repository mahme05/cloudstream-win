// src/components/Layout.tsx
// The persistent sidebar + content area wrapper.
// All pages render inside <Outlet /> (where the matched route goes).

import { Outlet, NavLink, useNavigate } from "react-router-dom";
import { useState } from "react";
import { useStore } from "../store";
import clsx from "clsx";

const NAV_ITEMS = [
  { to: "/", label: "Home", icon: "⊞" },
  { to: "/search", label: "Search", icon: "⌕" },
  { to: "/bookmarks", label: "Bookmarks", icon: "♥" },
  { to: "/history", label: "History", icon: "⏱" },
  { to: "/downloads", label: "Downloads", icon: "↓" },
  { to: "/plugins", label: "Plugins", icon: "⚙" },
];

export default function Layout() {
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const downloads = useStore((s) => s.downloads);
  const activeDownloads = downloads.filter((d) => d.status === "downloading").length;

  return (
    <div className="app-shell">
      {/* Sidebar */}
      <aside className={clsx("sidebar", { collapsed: !sidebarOpen })}>
        {/* Logo */}
        <div className="sidebar-logo">
          {sidebarOpen ? (
            <span className="logo-text">CloudStream</span>
          ) : (
            <span className="logo-icon">▶</span>
          )}
          <button
            className="collapse-btn"
            onClick={() => setSidebarOpen(!sidebarOpen)}
          >
            {sidebarOpen ? "◀" : "▶"}
          </button>
        </div>

        {/* Navigation */}
        <nav className="sidebar-nav">
          {NAV_ITEMS.map((item) => (
            <NavLink
              key={item.to}
              to={item.to}
              end={item.to === "/"}
              className={({ isActive }) =>
                clsx("nav-item", { active: isActive })
              }
            >
              <span className="nav-icon">{item.icon}</span>
              {sidebarOpen && (
                <span className="nav-label">
                  {item.label}
                  {item.to === "/downloads" && activeDownloads > 0 && (
                    <span className="badge">{activeDownloads}</span>
                  )}
                </span>
              )}
            </NavLink>
          ))}
        </nav>
      </aside>

      {/* Main content area */}
      <main className="main-content">
        <Outlet />
      </main>
    </div>
  );
}
