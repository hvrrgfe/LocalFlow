import { NavLink, Outlet } from "react-router-dom";

const navItems = [
  { path: "/chat", label: "对话" },
  { path: "/agents", label: "Agent" },
  { path: "/providers", label: "API 管理" },
  { path: "/runs", label: "运行日志" },
  { path: "/security", label: "安全设置" },
];

export function Layout() {
  return (
    <div className="app-layout">
      <aside className="sidebar">
        <div className="sidebar-header">
          <h2>LocalFlow</h2>
          <span className="sidebar-subtitle">本地 AI 工作流</span>
        </div>
        <nav className="sidebar-nav">
          {navItems.map((item) => (
            <NavLink
              key={item.path}
              to={item.path}
              end={item.path === "/agents"}
              className={({ isActive }) =>
                "nav-item" + (isActive ? " active" : "")
              }
            >
              {item.label}
            </NavLink>
          ))}
        </nav>
      </aside>
      <main className="main-content">
        <Outlet />
      </main>
    </div>
  );
}