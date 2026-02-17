import { useState, useRef, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

type Message = {
  role: "user" | "ai";
  text: string;
};

type Agent = {
  id: number;
  name: string;
  description: string;
  openclaw_task: string;
  schedule: string;
  status: string;
};

type Approval = {
  id: number;
  agent_id: number;
  content: string;
  status: string;
};

type Log = {
  id: number;
  agent_id: number;
  timestamp: string;
  message: string;
  level: string;
};

type EnvInfo = {
  os: string;
  has_node: boolean;
  has_pnpm: boolean;
  has_openclaw: boolean;
  has_ollama: boolean;
  has_playwright: boolean;
};

type SetupStep = "checking_env" | "installing_openclaw" | "llm_choice" | "api_key_input" | "ollama_check" | "phi3_pulling" | "creating_agent" | "done" | "error";

function App() {
  const [input, setInput] = useState("");
  const [messages, setMessages] = useState<Message[]>([
    { role: "ai", text: "👋 Hello! I am your Personaliz AI Assistant. Tell me to 'create agent trending' or 'create agent hashtag' to start!" },
  ]);
  const [agents, setAgents] = useState<Agent[]>([]);
  const [approvals, setApprovals] = useState<Approval[]>([]);
  const [logs, setLogs] = useState<Log[]>([]);
  const [view, setView] = useState<"chat" | "agents" | "approvals" | "logs" | "settings">("chat");
  const [isOpen, setIsOpen] = useState(true);
  const [apiKey, setApiKey] = useState<string>("");
  const [setupStep, setSetupStep] = useState<SetupStep | null>(null);
  const [setupError, setSetupError] = useState<string>("");
  const [pendingAgent, setPendingAgent] = useState<{ name: string, task: string, schedule: string } | null>(null);

  const chatEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    loadAgents();
    loadApprovals();
    loadLogs();
    loadSettings();
    const interval = setInterval(() => {
      loadApprovals();
      loadLogs();
    }, 5000);
    return () => clearInterval(interval);
  }, []);

  async function loadSettings() {
    try {
      const key = await invoke<string | null>("get_llm_settings");
      if (key) setApiKey(key);
    } catch (e) {
      console.error("Failed to load settings", e);
    }
  }

  async function saveApiKey() {
    try {
      await invoke("update_llm_settings", { key: apiKey });
      await invoke("log_event_cmd", { message: "LLM API Key updated. Switching to external model.", level: "Info" });
      alert("Settings saved!");
    } catch (e) {
      alert("Failed to save settings: " + e);
    }
  }

  async function loadAgents() {
    try {
      const data = await invoke<Agent[]>("get_agents");
      setAgents(data);
    } catch (e) {
      console.error("Failed to load agents", e);
    }
  }

  async function loadApprovals() {
    try {
      const data = await invoke<Approval[]>("get_approvals");
      setApprovals(data);
    } catch (e) {
      console.error("Failed to load approvals", e);
    }
  }

  async function loadLogs() {
    try {
      const data = await invoke<Log[]>("get_logs");
      setLogs(data);
    } catch (e) {
      console.error("Failed to load logs", e);
    }
  }

  async function handleApprove(id: number, approved: boolean) {
    try {
      await invoke("approve_request", { id, approved });
      loadApprovals();
      loadLogs();
    } catch (e) {
      console.error("Failed to approve", e);
    }
  }

  async function sendMessage() {
    if (!input.trim()) return;

    const userMsg = input;
    setMessages((m) => [...m, { role: "user", text: userMsg }]);
    setInput("");

    // Check if user is asking to create an agent
    if (userMsg.toLowerCase().includes("create agent") || userMsg.toLowerCase().includes("trending") || userMsg.toLowerCase().includes("hashtag")) {
      setMessages((m) => [...m, { role: "ai", text: "🔍 Checking environment for agent creation..." }]);
      startSetupFlow(userMsg);
      return;
    }

    // Normal chat flow
    try {
      let responseText = "";
      if (apiKey) {
        setMessages((m) => [...m, { role: "ai", text: "📡 Using external LLM model..." }]);
        const res = await fetch("https://api.openai.com/v1/chat/completions", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            "Authorization": `Bearer ${apiKey}`
          },
          body: JSON.stringify({
            model: "gpt-3.5-turbo",
            messages: [{ role: "user", content: userMsg }]
          })
        });
        const data = await res.json();
        responseText = data.choices[0].message.content;
      } else {
        const res = await fetch("http://localhost:11434/api/generate", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            model: "phi3",
            prompt: userMsg,
            stream: false,
          }),
        });
        const data = await res.json();
        responseText = data.response;
      }
      setMessages((m) => [...m, { role: "ai", text: responseText || "No response." }]);
    } catch (e) {
      setMessages((m) => [...m, { role: "ai", text: `⚠️ LLM Error: ${e}. Ensure Ollama is running if using local model.` }]);
      await invoke("log_event_cmd", { message: `LLM Error: ${e}`, level: "Error" });
    }
  }

  async function startSetupFlow(userMsg: string) {
    let agentParams = { name: "Custom Agent", task: userMsg, schedule: "Manual" };
    if (userMsg.toLowerCase().includes("trending")) {
      agentParams = { name: "Trending LinkedIn Agent", task: "Trending LinkedIn Post", schedule: "Daily" };
    } else if (userMsg.toLowerCase().includes("hashtag")) {
      agentParams = { name: "Hashtag Comment Agent", task: "LinkedIn #openclaw comment", schedule: "Hourly" };
    }

    setPendingAgent(agentParams);
    setSetupStep("checking_env");

    try {
      const env = await invoke<EnvInfo>("detect_env");
      if (!env.has_openclaw) {
        setSetupStep("installing_openclaw");
        await invoke("install_openclaw");
      }
      setSetupStep("llm_choice");
    } catch (e) {
      setSetupError(String(e));
      setSetupStep("error");
    }
  }

  async function handleLlmChoice(choice: "local" | "external") {
    if (choice === "external") {
      setSetupStep("api_key_input");
    } else {
      setSetupStep("ollama_check");
      try {
        const running = await invoke<boolean>("check_ollama");
        if (!running) {
          setSetupError("Ollama is not running. Please start Ollama and try again.");
          setSetupStep("error");
          return;
        }
        setSetupStep("phi3_pulling");
        await invoke("ensure_phi3");
        finishSetup();
      } catch (e) {
        setSetupError(String(e));
        setSetupStep("error");
      }
    }
  }

  async function handleApiKeySubmit(key: string) {
    setApiKey(key);
    await invoke("update_llm_settings", { key });
    finishSetup();
  }

  async function finishSetup() {
    if (!pendingAgent) return;
    setSetupStep("creating_agent");
    try {
      await invoke("create_agent", pendingAgent);
      setMessages((m) => [...m, { role: "ai", text: `✅ Agent '${pendingAgent.name}' created successfully after setup!` }]);
      setSetupStep("done");
      setPendingAgent(null);
      loadAgents();
      setTimeout(() => setSetupStep(null), 2000);
    } catch (e) {
      setSetupError(String(e));
      setSetupStep("error");
    }
  }

  if (!isOpen) {
    return (
      <div onClick={() => setIsOpen(true)} style={{ position: "fixed", bottom: "20px", right: "20px", width: "60px", height: "60px", background: "#3b82f6", borderRadius: "50%", display: "flex", alignItems: "center", justifyContent: "center", cursor: "pointer", boxShadow: "0 4px 12px rgba(0,0,0,0.3)", zIndex: 1000, fontSize: "30px" }}>
        🧠
      </div>
    );
  }

  return (
    <div style={{ height: "100vh", display: "flex", background: "#0f172a", color: "white", fontFamily: "Inter, sans-serif" }}>
      {/* Sidebar */}
      <div style={{ width: "240px", background: "#1e293b", padding: "20px", display: "flex", flexDirection: "column", gap: "10px" }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <h1 style={{ fontSize: "20px", fontWeight: "bold" }}>🧠 Personaliz</h1>
          <button onClick={() => setIsOpen(false)} style={{ background: "transparent", border: "none", color: "white", cursor: "pointer", fontSize: "18px" }}>×</button>
        </div>
        <button onClick={() => setView("chat")} style={sidebarButtonStyle(view === "chat")}>💬 Assistant</button>
        <button onClick={() => setView("agents")} style={sidebarButtonStyle(view === "agents")}>🤖 My Agents ({agents.length})</button>
        <button onClick={() => setView("approvals")} style={sidebarButtonStyle(view === "approvals")}>⏳ Approvals ({approvals.length})</button>
        <button onClick={() => setView("logs")} style={sidebarButtonStyle(view === "logs")}>📜 Activity Logs</button>
        <div style={{ flex: 1 }}></div>
        <button onClick={() => setView("settings")} style={sidebarButtonStyle(view === "settings")}>⚙️ Settings</button>
      </div>

      {/* Main Content */}
      <div style={{ flex: 1, display: "flex", flexDirection: "column", background: "#0f172a" }}>
        {view === "chat" && (
          <>
            <div style={{ flex: 1, overflowY: "auto", padding: "20px", display: "flex", flexDirection: "column", gap: "12px" }}>
              {messages.map((msg, i) => (
                <div key={i} style={{
                  alignSelf: msg.role === "user" ? "flex-end" : "flex-start",
                  background: msg.role === "user" ? "#3b82f6" : "#334155",
                  padding: "10px 14px", borderRadius: "12px", maxWidth: "80%"
                }}>
                  {msg.text}
                </div>
              ))}
              <div ref={chatEndRef}></div>
            </div>
            <div style={{ padding: "20px", display: "flex", gap: "10px", borderTop: "1px solid #334155" }}>
              <input value={input} onChange={(e) => setInput(e.target.value)}
                placeholder="Tell me to create an agent..."
                style={{ flex: 1, padding: "12px", borderRadius: "8px", border: "none", background: "#1e293b", color: "white" }} />
              <button onClick={sendMessage} style={{ padding: "12px 20px", borderRadius: "8px", border: "none", background: "#3b82f6", color: "white", fontWeight: "bold" }}>Send</button>
            </div>
          </>
        )}

        {view === "agents" && (
          <div style={{ padding: "30px" }}>
            <h2>🤖 My Active Agents</h2>
            <div style={{ display: "grid", gap: "15px", marginTop: "20px" }}>
              {agents.map(a => (
                <div key={a.id} style={{ background: "#1e293b", padding: "15px", borderRadius: "10px", border: "1px solid #334155" }}>
                  <div style={{ fontWeight: "bold" }}>{a.name}</div>
                  <div style={{ fontSize: "14px", opacity: 0.7 }}>{a.openclaw_task}</div>
                  <div style={{ marginTop: "10px", fontSize: "12px", background: "#065f46", display: "inline-block", padding: "2px 8px", borderRadius: "4px" }}>{a.status}</div>
                </div>
              ))}
              {agents.length === 0 && <div>No agents created yet. Ask the assistant to create one!</div>}
            </div>
          </div>
        )}

        {view === "approvals" && (
          <div style={{ padding: "30px" }}>
            <h2>⏳ Pending Approvals</h2>
            <div style={{ display: "grid", gap: "15px", marginTop: "20px" }}>
              {approvals.map(a => (
                <div key={a.id} style={{ background: "#1e293b", padding: "15px", borderRadius: "10px", border: "1px solid #334155" }}>
                  <div style={{ fontWeight: "bold" }}>Approval Request</div>
                  <p style={{ margin: "10px 0" }}>{a.content}</p>
                  <div style={{ display: "flex", gap: "10px" }}>
                    <button onClick={() => handleApprove(a.id, true)} style={{ background: "#10b981", color: "white", padding: "5px 15px", borderRadius: "5px", border: "none" }}>Approve & Post</button>
                    <button onClick={() => handleApprove(a.id, false)} style={{ background: "#ef4444", color: "white", padding: "5px 15px", borderRadius: "5px", border: "none" }}>Reject</button>
                  </div>
                </div>
              ))}
              {approvals.length === 0 && <div>No pending approvals.</div>}
            </div>
          </div>
        )}

        {view === "logs" && (
          <div style={{ padding: "30px" }}>
            <h2>📜 Activity Logs</h2>
            <div style={{ marginTop: "20px", display: "flex", flexDirection: "column", gap: "10px" }}>
              {logs.map(l => (
                <div key={l.id} style={{ fontSize: "13px", padding: "8px", borderBottom: "1px solid #334155" }}>
                  <span style={{ opacity: 0.5 }}>[{l.timestamp}]</span> <span style={{ color: l.level === "Error" ? "#ef4444" : "#10b981" }}>{l.level}</span>: {l.message}
                </div>
              ))}
              {logs.length === 0 && <div>No activity logged yet.</div>}
            </div>
          </div>
        )}

        {view === "settings" && (
          <div style={{ padding: "30px" }}>
            <h2>⚙️ System Settings</h2>
            <div style={{ marginTop: "20px", display: "flex", flexDirection: "column", gap: "20px" }}>
              <div style={{ background: "#1e293b", padding: "20px", borderRadius: "10px", border: "1px solid #334155" }}>
                <h3 style={{ marginTop: 0 }}>LLM Configuration</h3>
                <p style={{ opacity: 0.7, fontSize: "14px" }}>Provide an API key to switch from local Phi-3 to an external provider.</p>
                <div style={{ display: "flex", gap: "10px", marginTop: "15px" }}>
                  <input
                    type="password"
                    value={apiKey}
                    onChange={(e) => setApiKey(e.target.value)}
                    placeholder="Enter API Key (OpenAI/Claude)..."
                    style={{ flex: 1, padding: "10px", borderRadius: "5px", border: "none", background: "#0f172a", color: "white" }}
                  />
                  <button onClick={saveApiKey} style={{ padding: "10px 20px", background: "#3b82f6", border: "none", borderRadius: "5px", color: "white", fontWeight: "bold", cursor: "pointer" }}>Save</button>
                </div>
                {!apiKey && <p style={{ color: "#10b981", fontSize: "12px", marginTop: "10px" }}>✅ Currently using Local LLM (Phi-3)</p>}
                {apiKey && <p style={{ color: "#3b82f6", fontSize: "12px", marginTop: "10px" }}>📡 Currently using External Model Provider</p>}
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Setup Wizard Overlay */}
      {setupStep && (
        <div style={{ position: "fixed", inset: 0, background: "rgba(0,0,0,0.8)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 2000 }}>
          <div style={{ background: "#1e293b", padding: "30px", borderRadius: "15px", maxWidth: "400px", width: "90%", border: "1px solid #3b82f6", textAlign: "center" }}>
            <h2 style={{ marginBottom: "20px" }}>🚀 Agent Setup Wizard</h2>

            {setupStep === "checking_env" && <p>🔍 Checking your environment...</p>}
            {setupStep === "installing_openclaw" && <p>📦 Installing OpenClaw globally... This may take a moment.</p>}

            {setupStep === "llm_choice" && (
              <div>
                <p>Which LLM module would you like to use for this agent?</p>
                <div style={{ display: "flex", gap: "10px", marginTop: "20px" }}>
                  <button onClick={() => handleLlmChoice("local")} style={wizardButtonStyle}>🏠 Local (Phi-3)</button>
                  <button onClick={() => handleLlmChoice("external")} style={wizardButtonStyle}>📡 External (API Key)</button>
                </div>
              </div>
            )}

            {setupStep === "api_key_input" && (
              <div>
                <p>Please provide your API Key (OpenAI/Claude):</p>
                <input type="password" onKeyDown={(e) => e.key === "Enter" && handleApiKeySubmit((e.target as HTMLInputElement).value)}
                  style={{ width: "100%", padding: "10px", marginTop: "10px", borderRadius: "5px", border: "none", background: "#0f172a", color: "white" }}
                  placeholder="Paste key and press Enter" />
              </div>
            )}

            {setupStep === "ollama_check" && <p>🤖 Verifying Ollama is running...</p>}
            {setupStep === "phi3_pulling" && <p>📥 Pulling Phi-3 model... (This can take a few minutes if first time)</p>}
            {setupStep === "creating_agent" && <p>🤖 Finalizing agent creation...</p>}
            {setupStep === "done" && <p style={{ color: "#10b981" }}>✅ Everything is ready! Agent created.</p>}

            {setupStep === "error" && (
              <div>
                <p style={{ color: "#ef4444" }}>❌ Error during setup:</p>
                <p style={{ fontSize: "14px", opacity: 0.8 }}>{setupError}</p>
                <button onClick={() => setSetupStep(null)} style={{ ...wizardButtonStyle, marginTop: "15px" }}>Close</button>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

function sidebarButtonStyle(active: boolean) {
  return {
    padding: "10px 15px",
    borderRadius: "8px",
    border: "none",
    background: active ? "#3b82f6" : "transparent",
    color: "white",
    textAlign: "left" as const,
    cursor: "pointer",
    fontWeight: active ? "bold" : "normal",
  };
}

const wizardButtonStyle = {
  flex: 1,
  padding: "12px",
  borderRadius: "8px",
  border: "none",
  background: "#3b82f6",
  color: "white",
  fontWeight: "bold" as const,
  cursor: "pointer"
};

export default App;
