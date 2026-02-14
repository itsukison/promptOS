import { useEffect, useState } from "react";
import { OverlayView } from "./components/OverlayView";
import { SettingsView } from "./components/SettingsView";

function App() {
  const [route, setRoute] = useState<string>("/");

  useEffect(() => {
    // Get initial route from URL path
    const path = window.location.pathname;
    setRoute(path);
  }, []);

  // Route to appropriate view based on window
  if (route === "/overlay") {
    return <OverlayView />;
  }

  if (route === "/settings") {
    return <SettingsView />;
  }

  // Default: show settings (for development)
  return <SettingsView />;
}

export default App;
