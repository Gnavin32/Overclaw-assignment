import { useState } from "react";

type Message = {
  role: "user" | "ai";
  text: string;
};

function App() {
  const [input, setInput] = useState("");
  const [messages, setMessages] = useState<Message[]>([]);

  async function sendMessage() {
    if (!input) return;

    const userMsg = { role: "user", text: input };
    setMessages((m) => [...m, userMsg]);
    setInput("");

    const res = await fetch("http://localhost:11434/api/generate", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        model: "phi3",
        prompt: input,
        stream: false,
      }),
    });

    const data = await res.json();

    setMessages((m) => [
      ...m,
      { role: "ai", text: data.response },
    ]);
  }

  return (
    <div
    style={{
      height: "100vh",
      display: "flex",
      flexDirection: "column",
      background: "#0f172a",
      color: "white",
      fontFamily: "Segoe UI, sans-serif",
      padding: "15px",
    }}
  >
    <h2 style={{ marginBottom: "10px" }}>🤖 Personaliz AI Assistant</h2>

    {/* Chat Area */}
    <div
      style={{
        flex: 1,
        overflowY: "auto",
        background: "#1e293b",
        padding: "15px",
        borderRadius: "12px",
      }}
    >
      {messages.map((m, i) => (
        <div
          key={i}
          style={{
            marginBottom: "10px",
            display: "flex",
            justifyContent: m.role === "user" ? "flex-end" : "flex-start",
          }}
        >
          <div
            style={{
              maxWidth: "70%",
              padding: "10px 14px",
              borderRadius: "12px",
              background: m.role === "user" ? "#3b82f6" : "#334155",
            }}
          >
            {m.text}
          </div>
        </div>
      ))}
    </div>

    {/* Input Area */}
    <div style={{ display: "flex", marginTop: "12px" }}>
      <input
        value={input}
        onChange={(e) => setInput(e.target.value)}
        placeholder="Ask anything..."
        style={{
          flex: 1,
          padding: "12px",
          borderRadius: "10px",
          border: "none",
          outline: "none",
          fontSize: "14px",
        }}
      />

      <button
        onClick={sendMessage}
        style={{
          marginLeft: "10px",
          padding: "12px 18px",
          border: "none",
          borderRadius: "10px",
          background: "#3b82f6",
          color: "white",
          cursor: "pointer",
          fontWeight: "bold",
        }}
      >
        Send
      </button>
    </div>
  </div>
);