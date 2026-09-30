import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { useApplication } from "./bridge";
import { Overlay } from "./Overlay";
import { SettingsWindow } from "./SettingsWindow";
import "./styles.css";
const overlay = new URLSearchParams(location.search).has("overlay");
document.documentElement.classList.toggle("overlay-root", overlay);
function App() {
  const app = useApplication();
  return overlay ? (
    <Overlay state={app.state} act={app.act} error={app.error} />
  ) : (
    <SettingsWindow {...app} />
  );
}
createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
