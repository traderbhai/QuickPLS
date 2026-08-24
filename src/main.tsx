import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "@xyflow/react/dist/style.css";
import { App } from "./App";
import { installMultiModPackagedQualificationBridgeV1 } from "./native/multimodPackagedQualificationBridgeV1";

installMultiModPackagedQualificationBridgeV1();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
