import { useEffect, useState } from "react";
import { RuntimeService } from "../bindings/github.com/alexfedura/vora_mind/go-app";

type Project = { id: string; name: string; description: string };
type Result = { status: string; data?: unknown; error?: { code: string; message: string } | null };
type DatabaseStatus = { status?: string; path?: string; migrationCount?: number; foreignKeys?: boolean };

function App() {
  const [projects,setProjects]=useState<Project[]>([]);
  const [capabilities,setCapabilities]=useState<Array<{id:string;name:string;description:string}>>([]);
  const [databaseStatus,setDatabaseStatus]=useState<DatabaseStatus|null>(null);
  const [databaseStatusLoading,setDatabaseStatusLoading]=useState(true);
  const [databaseStatusError,setDatabaseStatusError]=useState("");
  const [selected,setSelected]=useState<Project|null>(null);
  const [name,setName]=useState("");
  const [description,setDescription]=useState("");
  const [apiStatus,setAPIStatus]=useState<{running?:boolean;address?:string;authentication?:string}>({});
  const [error,setError]=useState("");
  const [loading,setLoading]=useState(true);
  const [workflows,setWorkflows]=useState<Array<{id:string;name:string;status:string}>>([]);
  const [credentials,setCredentials]=useState<Array<{id:string;name:string;status:string;permissions:string[];projectIds:string[];createdAt:string;lastUsedAt?:string}>>([]);
  const [credentialName,setCredentialName]=useState("");
  const [credentialSecret,setCredentialSecret]=useState("");
  const [backups,setBackups]=useState<Array<{path:string;size:number;createdAt:string;valid:boolean}>>([]);
  const [selectedBackup,setSelectedBackup]=useState("");
  const [restoreValidation,setRestoreValidation]=useState("");
  const [activeProject,setActiveProject]=useState<Project|null>(null);
  const [workspaceRoots,setWorkspaceRoots]=useState<Array<{projectId:string;root:string;createdAt:string;updatedAt:string}>>([]);
  const [workspaceRoot,setWorkspaceRoot]=useState("");

  const run=async(operation:()=>unknown,onSuccess?:()=>void)=>{
    setError("");
    try{
      const result=await Promise.resolve(operation() as PromiseLike<unknown>);
      const normalized=result as Result;
      if(normalized.status==="failed"){setError(normalized.error?.message||"The operation failed.");return null}
      onSuccess?.();
      return normalized;
    }catch(caught){setError(caught instanceof Error?caught.message:String(caught));return null}
  };

  const refresh=async()=>{
    setLoading(true);
    const projectResult=await run(()=>RuntimeService.ListProjects());
    if(projectResult){
      const data=projectResult.data as {projects?:Project[]}|undefined;
      setProjects(data?.projects||[]);
    }
    const capabilityResult=await run(()=>RuntimeService.ListCapabilities());
    if(capabilityResult){
      const data=capabilityResult.data as {capabilities?:Array<{id:string;name:string;description:string}>}|undefined;
      setCapabilities(data?.capabilities||[]);
    }
    const contextResult=await run(()=>RuntimeService.GetProjectContext());
    if(contextResult){
      const data=contextResult.data as {active?:boolean;project?:Project|null}|undefined;
      setActiveProject(data?.active?data.project||null:null);
    }
    const rootsResult=await run(()=>RuntimeService.ListProjectFilesystemRoots());
    if(rootsResult){
      const data=rootsResult.data as {roots?:typeof workspaceRoots}|undefined;
      const roots=data?.roots||[];
      setWorkspaceRoots(roots);
      const activeId=contextResult?.data
        ? (contextResult.data as {active?:boolean;project?:Project|null}).project?.id
        : undefined;
      const activeRoot=activeId?roots.find(root=>root.projectId===activeId):undefined;
      setWorkspaceRoot(activeRoot?.root||"");
    }
    setLoading(false);
  };

  useEffect(()=>{void refresh()},[]);

  const refreshDatabaseStatus=async()=>{
    setDatabaseStatusLoading(true);
    setDatabaseStatusError("");
    try{
      const result=await Promise.resolve(RuntimeService.GetDatabaseStatus() as PromiseLike<Result>);
      if(result.status==="failed"){
        setDatabaseStatus(null);
        setDatabaseStatusError(result.error?.message||"Could not read database status.");
        return;
      }
      setDatabaseStatus((result.data||{}) as DatabaseStatus);
    }catch(caught){
      setDatabaseStatus(null);
      setDatabaseStatusError(caught instanceof Error?caught.message:String(caught));
    }finally{
      setDatabaseStatusLoading(false);
    }
  };

  useEffect(()=>{void refreshDatabaseStatus()},[]);

  const createProject=async()=>{
    if(!name.trim()){setError("Project name is required.");return}
    await run(()=>RuntimeService.CreateProject(name,description),()=>{setName("");setDescription("");void refresh()});
  };

  const selectProject=async(project:Project)=>{
    const result=await run(()=>RuntimeService.OpenProject(project.id));
    if(result){
      const data=result.data as {project:Project}|undefined;
      if(data?.project){setSelected(data.project);setActiveProject(data.project)}
      void refresh();
    }
  };

  const updateProject=async()=>{
    if(!selected)return;
    await run(()=>RuntimeService.UpdateProject(selected.id,selected.name,selected.description),()=>void refresh());
  };

  const deleteProject=async()=>{
    if(!selected)return;
    if(!window.confirm("Delete this project metadata? Workspace files will be preserved."))return;
    await run(()=>RuntimeService.DeleteProject(selected.id) as unknown as PromiseLike<Result>,()=>{
      setSelected(null);
      void refresh();
    });
  };

  const startAPI=async()=>{
    const status=await run(async()=>await RuntimeService.StartAPI() as unknown as Result);
    if(status){setAPIStatus(status as unknown as {running?:boolean;address?:string;authentication?:string})}
  };

  const stopAPI=async()=>{
    const status=await run(async()=>await RuntimeService.StopAPI() as unknown as Result);
    if(status){setAPIStatus({running:false})}
  };

  const refreshWorkflows=async()=>{
    const result=await run(()=>RuntimeService.ListWorkflows());
    if(result){
      const data=result.data as {workflows?:Array<{id:string;name:string;status:string}>}|undefined;
      setWorkflows(data?.workflows||[]);
    }
  };

  useEffect(()=>{void refreshWorkflows()},[]);

  const refreshCredentials=async()=>{
    const result=await run(()=>RuntimeService.ListCredentials() as unknown as PromiseLike<Result>);
    if(result){
      const data=result as unknown as {credentials?:typeof credentials};
      setCredentials(data.credentials||[]);
    }
  };

  useEffect(()=>{void refreshCredentials()},[]);

  const createCredential=async()=>{
    if(!credentialName.trim()){setError("Credential name is required.");return}
    const result=await run(()=>RuntimeService.CreateCredential(
      credentialName.trim(),
      JSON.stringify(["project.read","capabilities.discover"]),
      JSON.stringify([])
    ) as unknown as PromiseLike<Result>);
    if(result){
      const data=result as unknown as {credential?:unknown;secret?:string};
      setCredentialSecret(typeof data.secret==="string"?data.secret:"");
      setCredentialName("");
      await refreshCredentials();
    }
  };

  const revokeCredential=async(id:string)=>{
    if(!window.confirm("Revoke this credential? This action cannot be undone."))return;
    await run(()=>RuntimeService.RevokeCredential(id) as unknown as PromiseLike<Result>,()=>void refreshCredentials());
  };

  const rotateCredential=async(id:string)=>{
    if(!window.confirm("Rotate this credential? The old secret will stop working immediately."))return;
    const result=await run(()=>RuntimeService.RotateCredential(id) as unknown as PromiseLike<Result>);
    if(result){
      const data=result as unknown as {secret?:string};
      setCredentialSecret(typeof data.secret==="string"?data.secret:"");
      await refreshCredentials();
    }
  };

  const refreshBackups=async()=>{
    const result=await run(()=>RuntimeService.ListBackups() as unknown as PromiseLike<Result>);
    if(result){
      const data=result as unknown as {backups?:typeof backups};
      setBackups(data.backups||[]);
    }
  };

  useEffect(()=>{void refreshBackups()},[]);

  const createBackup=async()=>{
    await run(()=>RuntimeService.CreateBackup() as unknown as PromiseLike<Result>,()=>void refreshBackups());
  };

  const validateBackup=async(path:string)=>{
    setSelectedBackup(path);
    setRestoreValidation("");
    const result=await run(()=>RuntimeService.ValidateBackup(path) as unknown as PromiseLike<Result>);
    if(result){
      const data=result as unknown as {valid?:boolean};
      setRestoreValidation(data.valid?"Valid backup":"Validation failed");
    }
  };

  const restoreBackup=async()=>{
    if(!selectedBackup){setError("Select a backup first.");return}
    if(restoreValidation!=="Valid backup"){setError("Validate the backup before restore.");return}
    if(!window.confirm("Restore this database backup? The live database will be replaced and a recovery copy will be preserved."))return;
    await run(()=>RuntimeService.RestoreBackup(selectedBackup) as unknown as PromiseLike<Result>,()=>{
      setRestoreValidation("Restore completed");
      void refresh();
      void refreshWorkflows();
      void refreshBackups();
    });
  };

  return (
    <main className="vora-foundation">
      <header><h1>Vora Mind</h1><p>Go/Wails capability runtime</p></header>
      {error&&<div className="error" role="alert">{error}</div>}
      <section>
        <h2>Database Status</h2>
        {databaseStatusLoading?<p>Loading database status...</p>:databaseStatusError?(
          <p className="error" role="alert">Database status error: {databaseStatusError}</p>
        ):databaseStatus&&(
          <>
            <p>Status: {databaseStatus.status||"unknown"}</p>
            <p>Database path: <code>{databaseStatus.path||"unknown"}</code></p>
          </>
        )}
        <button onClick={()=>void refreshDatabaseStatus()} disabled={databaseStatusLoading}>Refresh database status</button>
      </section>
      <section>
        <h2>API Status</h2>
        <p>Status: {apiStatus.running?"running":"disabled"}</p>
        {apiStatus.address&&<p>Address: {apiStatus.address}</p>}
        {apiStatus.authentication&&<p>Authentication: {apiStatus.authentication}</p>}
        <button onClick={()=>void startAPI()}>Enable loopback API</button>{" "}
        <button onClick={()=>void stopAPI()} disabled={!apiStatus.running}>Disable API</button>
      </section>
      <section>
        <h2>Projects</h2>
        {loading?<p>Loading...</p>:projects.length===0?<p>No projects yet.</p>:(
          <ul>{projects.map(project=><li key={project.id}><button onClick={()=>void selectProject(project)}>{project.name}</button></li>)}</ul>
        )}
        <div className="project-form">
          <input aria-label="Project name" value={name} onChange={event=>setName(event.target.value)} placeholder="Project name"/>
          <input aria-label="Project description" value={description} onChange={event=>setDescription(event.target.value)} placeholder="Description"/>
          <button onClick={()=>void createProject()} disabled={!name.trim()}>Create</button>
        </div>
        {selected&&(
          <div>
            <h3>Project details</h3>
            <input aria-label="Selected project name" value={selected.name} onChange={event=>setSelected({...selected,name:event.target.value})}/>
            <input aria-label="Selected project description" value={selected.description} onChange={event=>setSelected({...selected,description:event.target.value})}/>
            <button onClick={()=>void updateProject()}>Update</button>
            <button onClick={()=>void deleteProject()}>Delete project</button>
            <h3>Workspace root</h3>
            <p>Active project: {activeProject?.name||"none"}</p>
            <input aria-label="Workspace root" value={workspaceRoot} onChange={event=>setWorkspaceRoot(event.target.value)} placeholder="Absolute workspace root"/>
            <button onClick={()=>void run(()=>RuntimeService.UpdateProjectFilesystemRoot(selected.id,workspaceRoot),()=>void refresh())}>Update root</button>{" "}
            <button onClick={()=>{
              if(!window.confirm("Remove this project workspace root? Workspace files will be preserved."))return;
              void run(()=>RuntimeService.RemoveProjectFilesystemRoot(selected.id),()=>{
                setWorkspaceRoot("");
                void refresh();
              });
            }}>Remove root</button>
          </div>
        )}
        {workspaceRoots.length>0&&(
          <ul>{workspaceRoots.map(root=>(
            <li key={root.projectId}>
              <code>{root.projectId}</code>: {root.root}
            </li>
          ))}</ul>
        )}
      </section>
      <section>
        <h2>Capability Explorer</h2>
        {capabilities.length===0?<p>No capabilities loaded.</p>:(
          <ul>{capabilities.map(capability=><li key={capability.id}><code>{capability.id}</code><br/><small>{capability.description}</small></li>)}</ul>
        )}
      </section>
      <section>
        <h2>Workflow Manager</h2>
        {workflows.length===0?<p>No workflows yet.</p>:(
          <ul>{workflows.map(workflow=>(
            <li key={workflow.id}>
              <strong>{workflow.name}</strong> — {workflow.status}{" "}
              <button onClick={()=>void run(()=>RuntimeService.StartWorkflow(workflow.id),()=>void refreshWorkflows())}>Start</button>{" "}
              <button onClick={()=>void run(()=>RuntimeService.CancelWorkflow(workflow.id),()=>void refreshWorkflows())}>Cancel</button>{" "}
              <button onClick={()=>void run(()=>RuntimeService.ResumeWorkflow(workflow.id),()=>void refreshWorkflows())}>Resume</button>
            </li>
          ))}</ul>
        )}
      </section>
      <section>
        <h2>Credential Management</h2>
        <div>
          <input aria-label="Credential name" value={credentialName} onChange={event=>setCredentialName(event.target.value)} placeholder="Credential name"/>
          <button onClick={()=>void createCredential()} disabled={!credentialName.trim()}>Create</button>
        </div>
        {credentialSecret&&(
          <div role="alert">
            <strong>One-time secret</strong>
            <pre>{credentialSecret}</pre>
            <button onClick={async()=>{
              await navigator.clipboard.writeText(credentialSecret);
              setError("");
            }}>Copy secret</button>{" "}
            <button onClick={()=>setCredentialSecret("")}>Dismiss</button>
          </div>
        )}
        {credentials.length===0?<p>No credentials yet.</p>:(
          <ul>{credentials.map(credential=>(
            <li key={credential.id}>
              <strong>{credential.name}</strong> — {credential.status}<br/>
              <small>Created: {credential.createdAt} | Last used: {credential.lastUsedAt||"never"} | Permissions: {credential.permissions.join(", ")}</small><br/>
              {credential.status==="active"&&<>
                <button onClick={()=>void rotateCredential(credential.id)}>Rotate</button>{" "}
                <button onClick={()=>void revokeCredential(credential.id)}>Revoke</button>
              </>}
            </li>
          ))}</ul>
        )}
      </section>
      <section>
        <h2>Backup and Restore</h2>
        <button onClick={()=>void createBackup()}>Create backup</button>
        {backups.length===0?<p>No backups yet.</p>:(
          <ul>{backups.map(backup=>(
            <li key={backup.path}>
              <button onClick={()=>void validateBackup(backup.path)}>{backup.path}</button>
              <small> {backup.valid?"valid":"invalid"} — {backup.createdAt}</small>
            </li>
          ))}</ul>
        )}
        {selectedBackup&&<p>Selected: {selectedBackup}</p>}
        {restoreValidation&&<p>{restoreValidation}</p>}
        <button onClick={()=>void restoreBackup()} disabled={!selectedBackup||restoreValidation!=="Valid backup"}>Restore selected backup</button>
      </section>
    </main>
  );
}

export default App;
