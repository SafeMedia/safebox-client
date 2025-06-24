import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { download } from "@/backend/logic";
import { UploadPayload } from "@/types/upload-file-event";
import { AccountUser } from "@/types/account-user";
import { ToastPayload } from "@/types/payloads";
import { Button } from "./ui/button";

export default function Dashboard() {
    const accountRef = useRef<AccountUser | null>(null);
    const [isClientRunning, setIsClientRunning] = useState(false);
    const [antPort, setAntPort] = useState<number>(8081);
    const [anttpPort, setAnttpPort] = useState<number>(8082);
    const [dwebPort, setDwebPort] = useState<number>(8083);
    const [websocketPort, setWebsocketPort] = useState<number>(8084);

    // Fetch ports from backend
    const refreshPorts = async () => {
        try {
            const [ant, anttp, dweb, websocket] = await invoke<
                [number, number, number, number]
            >("get_ports");
            setAntPort(ant);
            setAnttpPort(anttp);
            setDwebPort(dweb);
            setWebsocketPort(websocket);
        } catch (e) {
            toast.error("Failed to get ports", {
                description: (e as Error).toString(),
            });
        }
    };

    useEffect(() => {
        refreshPorts();

        // Event listeners
        const unlistenDownload = listen<string>(
            "download-file",
            async (event) => {
                toast("Download Request", { description: event.payload });
                try {
                    await download(event.payload, event.payload);
                } catch {
                    toast.error("Download Failed", {
                        description: `Failed to download: ${event.payload}`,
                    });
                }
            }
        );

        const unlistenUpload = listen<UploadPayload>(
            "upload-file",
            async (event) => {
                if (!accountRef.current) {
                    toast.error("Not Signed In", {
                        description: `Upload requested but you are not signed in`,
                    });
                    return;
                }
                const { name, success, error, xorname } = event.payload;
                if (success) {
                    toast(`File '${name}' Uploaded`, { description: xorname });
                } else if (error) {
                    toast.error("Upload Failed", {
                        description: "Something went wrong during upload.",
                    });
                } else {
                    toast.error("Unknown Upload Status", {
                        description: `Could not determine result for "${name}"`,
                    });
                }
            }
        );

        const unlistenToast = listen<ToastPayload>("show-toast", (event) => {
            const { title, description } = event.payload;
            toast(title, { description });
        });

        return () => {
            unlistenDownload.then((fn) => fn());
            unlistenUpload.then((fn) => fn());
            unlistenToast.then((fn) => fn());
        };
    }, []);

    function hasPortConflict(...ports: number[]) {
        const set = new Set(ports);
        return set.size !== ports.length; // true if duplicates exist
    }

    // Toggles the client on/off by invoking Rust commands
    const toggleClient = async () => {
        if (hasPortConflict(antPort, anttpPort, dwebPort)) {
            toast.error(
                "Port conflict detected! Please use unique ports for ant, anttp, and dweb."
            );
            return;
        }

        try {
            if (!isClientRunning) {
                await invoke("start_server");
                toast.success("Server started");
                setIsClientRunning(true);
            } else {
                await invoke("stop_server");
                toast.info("Server stopped");
                setIsClientRunning(false);
            }
        } catch (e: any) {
            toast.error("Client action failed", { description: e.toString() });
        }
    };

    // Update port and revert input if setting port fails or conflicts
    const updateAntPort = async () => {
        if (antPort < 1 || antPort > 65535) {
            toast.error("Invalid ant port number");
            refreshPorts();
            return;
        }
        if (hasPortConflict(antPort, anttpPort, dwebPort, websocketPort)) {
            toast.error("Port conflict detected! Please use unique ports.");
            refreshPorts();
            return;
        }
        try {
            await invoke("set_ant_port", { port: antPort });
            toast.success(`ant port set to ${antPort}`);
        } catch (e: any) {
            toast.error("Failed to set ant port", {
                description: e.toString(),
            });
            await refreshPorts();
        }
    };

    const updateAnttpPort = async () => {
        if (anttpPort < 1 || anttpPort > 65535) {
            toast.error("Invalid anttp port number");
            refreshPorts();
            return;
        }
        if (hasPortConflict(antPort, anttpPort, dwebPort, websocketPort)) {
            toast.error("Port conflict detected! Please use unique ports.");
            refreshPorts();
            return;
        }
        try {
            await invoke("set_anttp_port", { port: anttpPort });
            toast.success(`anttp port set to ${anttpPort}`);
        } catch (e: any) {
            toast.error("Failed to set anttp port", {
                description: e.toString(),
            });
            await refreshPorts();
        }
    };

    const updateDwebPort = async () => {
        if (dwebPort < 1 || dwebPort > 65535) {
            toast.error("Invalid dweb port number");
            refreshPorts();
            return;
        }
        if (hasPortConflict(antPort, anttpPort, dwebPort, websocketPort)) {
            toast.error("Port conflict detected! Please use unique ports.");
            refreshPorts();
            return;
        }
        try {
            await invoke("set_dweb_port", { port: dwebPort });
            toast.success(`dweb port set to ${dwebPort}`);
        } catch (e: any) {
            toast.error("Failed to set dweb port", {
                description: e.toString(),
            });
            await refreshPorts();
        }
    };

    const updateWebsocketPort = async () => {
        if (dwebPort < 1 || dwebPort > 65535) {
            toast.error("Invalid websocket port number");
            refreshPorts();
            return;
        }
        if (hasPortConflict(antPort, anttpPort, dwebPort, websocketPort)) {
            toast.error("Port conflict detected! Please use unique ports.");
            refreshPorts();
            return;
        }
        try {
            await invoke("set_websocket_port", { port: websocketPort });
            toast.success(`websocket port set to ${websocketPort}`);
        } catch (e: any) {
            toast.error("Failed to set websocket port", {
                description: e.toString(),
            });
            await refreshPorts();
        }
    };

    return (
        <div className="p-4 space-y-2">
            <div className="flex flex-row gap-2 items-center">
                <Button onClick={toggleClient} className="cursor-pointer">
                    {isClientRunning ? "Stop Server" : "Start Server"}
                </Button>
            </div>

            {/* ant port */}
            <div className="flex flex-row gap-2 items-center">
                <input
                    type="number"
                    value={antPort}
                    disabled={isClientRunning}
                    onChange={(e) => setAntPort(Number(e.target.value))}
                    className="border rounded p-1 w-24"
                    min={1}
                    max={65535}
                />
                <Button onClick={updateAntPort} disabled={isClientRunning}>
                    Set Ant port
                </Button>
            </div>

            {/* anttp port */}
            <div className="flex flex-row gap-2 items-center">
                <input
                    type="number"
                    value={anttpPort}
                    disabled={isClientRunning}
                    onChange={(e) => setAnttpPort(Number(e.target.value))}
                    className="border rounded p-1 w-24"
                    min={1}
                    max={65535}
                />
                <Button onClick={updateAnttpPort} disabled={isClientRunning}>
                    Set AntTP port
                </Button>
            </div>

            {/* dweb port */}
            <div className="flex flex-row gap-2 items-center">
                <input
                    type="number"
                    value={dwebPort}
                    disabled={isClientRunning}
                    onChange={(e) => setDwebPort(Number(e.target.value))}
                    className="border rounded p-1 w-24"
                    min={1}
                    max={65535}
                />
                <Button onClick={updateDwebPort} disabled={isClientRunning}>
                    Set DWeb port
                </Button>
            </div>

            {/* websocket port */}
            <div className="flex flex-row gap-2 items-center">
                <input
                    type="number"
                    value={websocketPort}
                    disabled={isClientRunning}
                    onChange={(e) => setWebsocketPort(Number(e.target.value))}
                    className="border rounded p-1 w-24"
                    min={1}
                    max={65535}
                />
                <Button
                    onClick={updateWebsocketPort}
                    disabled={isClientRunning}
                >
                    Set Websocket port
                </Button>
            </div>
        </div>
    );
}
