import { useEffect, useState } from "react";
import { RuntimeService } from "../bindings/github.com/alexfedura/vora_mind/go-app";

interface CapabilityResult {
  executionId: string;
  capabilityId: string;
  status: "succeeded" | "failed";
  data?: unknown;
  error?: { code: string; message: string };
  durationMs: number;
}

function asResult(value: unknown): CapabilityResult {
  return value as CapabilityResult;
}

function App() {
  const [info, setInfo] = useState<CapabilityResult | null>(null);
  const [database, setDatabase] = useState<CapabilityResult | null>(null);
  const [projects, setProjects] = useState<CapabilityResult | null>(null);
  const [capabilities, setCapabilities] = useState<CapabilityResult | null>(null);
  const [projectName, setProjectName] = useState("");
  const [projectDescription, setProjectDescription] = useState("");
  const [error, setError] = useState("");

  const run = async (operation: () => Promise<unknown>) => {
    setError("");
    try {
      const result = asResult(await operation());
      if (result.status === "failed") {
        setError(result.error?.message ?? "The operation failed.");
      }
      return result;
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
      return null;
    }
  };

  const refreshProjects = async () => {
    const result = await run(() => RuntimeService.ListProjects());
    if (result) setProjects(result);
  };

  const refresh = async () => {
    const infoResult = await run(() => RuntimeService.GetApplicationInfo());
    const databaseResult = await run(() => RuntimeService.GetDatabaseStatus());
    const capabilityResult = await run(() => RuntimeService.ListCapabilities());
    if (infoResult) setInfo(infoResult);
    if (databaseResult) setDatabase(databaseResult);
    if (capabilityResult) setCapabilities(capabilityResult);
    await refreshProjects();
  };

  useEffect(() => {
    void refresh();
  }, []);

  const createProject = async () => {
    const result = await run(() => RuntimeService.CreateProject(projectName, projectDescription));
    if (result?.status === "succeeded") {
      setProjectName("");
      setProjectDescription("");
      await refreshProjects();
    }
  };

  const projectData = (projects?.data as { projects?: Array<{ id: string; name: string; description: string }> } | undefined)?.projects ?? [];
  const databaseData = database?.data as { status?: string; migrationCount?: number; foreignKeys?: boolean } | undefined;
  const infoData = info?.data as { applicationName?: string; applicationVersion?: string; wailsVersion?: string; registeredCapabilityCount?: number } | undefined;
  const capabilityData = (capabilities?.data as { capabilities?: Array<{ id: string; name: string; description: string }> } | undefined)?.capabilities ?? [];

  return (
    <main className="vora-foundation">
      <header>
        <h1>Vora Mind</h1>
        <p>Go/Wails capability runtime foundation</p>
      </header>

      {error && <div className="error" role="alert">{error}</div>}

      <section>
        <h2>Application</h2>
        <dl>
          <dt>Name</dt><dd>{infoData?.applicationName ?? "Loading..."}</dd>
          <dt>Version</dt><dd>{infoData?.applicationVersion ?? "Loading..."}</dd>
          <dt>Wails</dt><dd>{infoData?.wailsVersion ?? "Loading..."}</dd>
          <dt>Capabilities</dt><dd>{infoData?.registeredCapabilityCount ?? "Loading..."}</dd>
        </dl>
      </section>

      <section>
        <h2>Database</h2>
        <p>Status: {databaseData?.status ?? "Loading..."}</p>
        <p>Migrations: {databaseData?.migrationCount ?? "Loading..."}</p>
        <p>Foreign keys: {databaseData?.foreignKeys ? "enabled" : "unknown"}</p>
      </section>

      <section>
        <h2>Projects</h2>
        <div className="project-form">
          <input aria-label="Project name" value={projectName} onChange={(event) => setProjectName(event.target.value)} placeholder="Project name" />
          <input aria-label="Project description" value={projectDescription} onChange={(event) => setProjectDescription(event.target.value)} placeholder="Description" />
          <button onClick={() => void createProject()} disabled={!projectName.trim()}>Create project</button>
        </div>
        {projectData.length === 0 ? <p>No projects yet.</p> : (
          <ul>
            {projectData.map((project) => <li key={project.id}><strong>{project.name}</strong>{project.description ? ` — ${project.description}` : ""}</li>)}
          </ul>
        )}
      </section>

      <section>
        <h2>Registered capabilities</h2>
        {capabilityData.length === 0 ? <p>Loading...</p> : (
          <ul>
            {capabilityData.map((capability) => <li key={capability.id}><code>{capability.id}</code> — {capability.name}</li>)}
          </ul>
        )}
      </section>
    </main>
  );
}

export default App;
