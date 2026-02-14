import { useState, useEffect } from "react";
import { storeApiKey, retrieveApiKey } from "../lib/commands";
import { useSupabase } from "../hooks/useSupabase";

export function SettingsView() {
  const [activeTab, setActiveTab] = useState<"general" | "account">("general");

  return (
    <div className="settings-container">
      <div className="settings-tabs">
        <button
          className={activeTab === "general" ? "active" : ""}
          onClick={() => setActiveTab("general")}
        >
          General
        </button>
        <button
          className={activeTab === "account" ? "active" : ""}
          onClick={() => setActiveTab("account")}
        >
          Account
        </button>
      </div>

      {activeTab === "general" && <GeneralSettings />}
      {activeTab === "account" && <AccountSettings />}
    </div>
  );
}

function GeneralSettings() {
  const [apiKey, setApiKey] = useState("");
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    retrieveApiKey().then((key) => {
      if (key) setApiKey(key);
    });
  }, []);

  const handleSave = async () => {
    await storeApiKey(apiKey);
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  };

  return (
    <div className="settings-section">
      <h3>Gemini API Key</h3>
      <p className="settings-hint">
        Get your API key from{" "}
        <a href="https://aistudio.google.com" target="_blank" rel="noreferrer">
          aistudio.google.com
        </a>
      </p>
      <div className="settings-row">
        <input
          type="password"
          value={apiKey}
          onChange={(e) => setApiKey(e.target.value)}
          placeholder="Enter your Gemini API key"
        />
        <button onClick={handleSave} className="btn-primary">
          {saved ? "Saved" : "Save"}
        </button>
      </div>
    </div>
  );
}

function AccountSettings() {
  const { user, signIn, signUp, signOut, getRemainingTokens, loading } =
    useSupabase();
  const [tokens, setTokens] = useState(0);
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);

  useEffect(() => {
    if (user) {
      getRemainingTokens().then(setTokens);
    }
  }, [user, getRemainingTokens]);

  const handleSignIn = async () => {
    setError("");
    setIsSubmitting(true);
    const { error } = await signIn(email, password);
    if (error) setError(error.message);
    setIsSubmitting(false);
  };

  const handleSignUp = async () => {
    setError("");
    setIsSubmitting(true);
    const { error } = await signUp(email, password);
    if (error) setError(error.message);
    setIsSubmitting(false);
  };

  if (loading) {
    return (
      <div className="settings-section">
        <p>Loading...</p>
      </div>
    );
  }

  if (!user) {
    return (
      <div className="settings-section">
        <h3>Sign In</h3>
        <input
          type="email"
          value={email}
          onChange={(e) => setEmail(e.target.value)}
          placeholder="Email"
        />
        <input
          type="password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          placeholder="Password"
        />
        {error && (
          <p style={{ color: "#ff6b6b", fontSize: 12, marginBottom: 8 }}>
            {error}
          </p>
        )}
        <div className="btn-group">
          <button
            onClick={handleSignIn}
            className="btn-primary"
            disabled={isSubmitting}
          >
            Sign In
          </button>
          <button
            onClick={handleSignUp}
            className="btn-secondary"
            disabled={isSubmitting}
          >
            Sign Up
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="settings-section">
      <h3>Account</h3>
      <p>Signed in as: {user.email}</p>
      <div className="token-display">
        <span className="token-count">{tokens.toLocaleString()}</span>
        <span className="token-label">tokens remaining</span>
      </div>
      <button onClick={signOut} className="btn-secondary">
        Sign Out
      </button>
    </div>
  );
}
