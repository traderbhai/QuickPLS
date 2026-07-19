import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
export default defineConfig({
    plugins: [react()],
    clearScreen: false,
    server: { port: 1420, strictPort: true, host: "127.0.0.1" },
    build: {
        rollupOptions: {
            output: {
                manualChunks(id) {
                    if (!id.includes("node_modules"))
                        return undefined;
                    if (id.includes("@xyflow/react") || id.includes("@reactflow"))
                        return "flow";
                    if (id.includes("lucide-react"))
                        return "icons";
                    if (id.includes("@tauri-apps"))
                        return "tauri";
                    return "vendor";
                },
            },
        },
    },
});
