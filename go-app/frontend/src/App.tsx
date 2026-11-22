import { useEffect, useState } from "react";
import { RuntimeService } from "../bindings/github.com/alexfedura/vora_mind/go-app";

type Project = { id: string; name: string; description: string };
type Result = { status: string; data?: unknown; error?: { code: string; message: string } | null };

function App() {
  const [projects,setProjects]=useState<Project[]>([]);
  const [capabilities,setCapabilities]=useState<Array<{id:string;name:string;description:string}>>([]);
  const [selected,setSelected]=useState<Project|null>(null);
  const [name,setName]=useState("");
  const [description,setDescription]=useState("");
  const [apiStatus,setAPIStatus]=useState<{running?:boolean;address?:string;authentication?:string}>({});
  const [error,setError]=useState("");
  const [loading,setLoading]=useState(true);
  const [workflows,setWorkflows]=useState<Array<{id:string;name:string;status:string}>>([]);

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
    setLoading(false);
  };

  useEffect(()=>{void refresh()},[]);

  const createProject=async()=>{
    if(!name.trim()){setError("Project name is required.");return}
    await run(()=>RuntimeService.CreateProject(name,description),()=>{setName("");setDescription("");void refresh()});
  };

  const selectProject=async(project:Project)=>{
    const result=await run(()=>RuntimeService.GetProject(project.id));
    if(result){setSelected((result.data as {project:Project}).project)}
  };

  const updateProject=async()=>{
    if(!selected)return;
    await run(()=>RuntimeService.UpdateProject(selected.id,selected.name,selected.description),()=>void refresh());
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

  return (
    <main className="vora-foundation">
      <header><h1>Vora Mind</h1><p>Go/Wails capability runtime</p></header>
      {error&&<div className="error" role="alert">{error}</div>}
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
          </div>
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
    </main>
  );
}

export default App;
