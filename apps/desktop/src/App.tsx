import { Routes, Route, Navigate } from "react-router-dom";
import { Layout } from "./components/Layout";
import { ErrorBoundary } from "./components/ErrorBoundary";
import AgentListPage from "./pages/AgentListPage";
import AgentEditorPage from "./pages/AgentEditorPage";
import WorkflowEditorPage from "./pages/WorkflowEditorPage";
import ApiManagementPage from "./pages/ApiManagementPage";
import RunLogsPage from "./pages/RunLogsPage";
import SecuritySettingsPage from "./pages/SecuritySettingsPage";
import "./index.css";

function App() {
  return (
    <ErrorBoundary>
      <Routes>
        <Route element={<Layout />}>
          <Route path="/" element={<Navigate to="/agents" replace />} />
          <Route path="/agents" element={<AgentListPage />} />
          <Route path="/agents/new" element={<AgentEditorPage />} />
          <Route path="/agents/:id" element={<AgentEditorPage />} />
          <Route path="/agents/:id/workflow" element={<WorkflowEditorPage />} />
          <Route path="/providers" element={<ApiManagementPage />} />
          <Route path="/runs" element={<RunLogsPage />} />
          <Route path="/runs/:workflowId" element={<RunLogsPage />} />
          <Route path="/security" element={<SecuritySettingsPage />} />
        </Route>
      </Routes>
    </ErrorBoundary>
  );
}

export default App;
