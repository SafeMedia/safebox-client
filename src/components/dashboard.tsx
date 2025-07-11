import { useEffect, useRef, useState } from "react";
import { toast } from "react-toastify";
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
    const [portToKill, setPortToKill] = useState<number>(0);

    const checkServerRunning = async () => {
        try {
            const running = await invoke<boolean>("is_server_running");
            setIsClientRunning(running);
        } catch {
            setIsClientRunning(false);
        }
    };

    const showVersion = async (binary: string) => {
        try {
            const version = await invoke<string>("get_binary_version", {
                binaryName: binary,
            });
            toast.success(`${binary} version: ${version}`);
        } catch (e: any) {
            toast.error(`Failed to get version for ${binary}: ${e}`);
        }
    };

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
            toast("Failed to get ports" + ": " + (e as Error).toString());
        }
    };

    useEffect(() => {
        checkServerRunning();
        refreshPorts();

        const unlistenDownload = listen<string>(
            "download-file",
            async (event) => {
                toast("Download Request" + ": " + event.payload);
                try {
                    await download(event.payload, event.payload);
                } catch {
                    toast(
                        "Download Failed" +
                            ": " +
                            `Failed to download: ${event.payload}`
                    );
                }
            }
        );

        const unlistenUpload = listen<UploadPayload>(
            "upload-file",
            async (event) => {
                if (!accountRef.current) {
                    toast(
                        "Not Signed In" +
                            ": " +
                            `Upload requested but you are not signed in`
                    );

                    return;
                }
                const { name, success, error, xorname } = event.payload;
                if (success) {
                    toast(`File '${name}' Uploaded` + ": " + xorname);
                } else if (error) {
                    toast(
                        "Upload Failed" +
                            ": " +
                            "Something went wrong during upload."
                    );
                } else {
                    toast(
                        "Unknown Upload Status" +
                            ": " +
                            `Could not determine result for "${name}"`
                    );
                }
            }
        );

        const unlistenToast = listen<ToastPayload>("show-toast", (event) => {
            const { title, description } = event.payload;
            toast(title + ": " + description);
        });

        return () => {
            unlistenDownload.then((fn) => fn());
            unlistenUpload.then((fn) => fn());
            unlistenToast.then((fn) => fn());
        };
    }, []);

    function hasPortConflict(...ports: number[]) {
        const set = new Set(ports);
        return set.size !== ports.length;
    }

    const toggleClient = async () => {
        try {
            if (!isClientRunning) {
                const [savedAnt, savedAnttp, savedDweb, savedWs] = await invoke<
                    [number, number, number, number]
                >("get_ports");

                if (
                    antPort !== savedAnt ||
                    anttpPort !== savedAnttp ||
                    dwebPort !== savedDweb ||
                    websocketPort !== savedWs
                ) {
                    setAntPort(savedAnt);
                    setAnttpPort(savedAnttp);
                    setDwebPort(savedDweb);
                    setWebsocketPort(savedWs);
                    toast.info("Ports reset to saved configuration.");
                }

                if (hasPortConflict(savedAnt, savedAnttp, savedDweb, savedWs)) {
                    toast.error("Port conflict detected in saved ports!");
                    return;
                }

                await invoke("start_server");
                toast.success("Server started");
                setIsClientRunning(true);
            } else {
                await invoke("stop_server");
                toast.info("Server stopped");
                setIsClientRunning(false);
            }
        } catch (e: any) {
            toast("Client action failed" + ": " + e.toString());
        }
    };

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
            toast("Failed to set ant port" + ": " + e.toString());

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
            toast("Failed to set anttp port" + ": " + e.toString());

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
            toast("Failed to set dweb port" + ": " + e.toString());

            await refreshPorts();
        }
    };

    const updateWebsocketPort = async () => {
        if (websocketPort < 1 || websocketPort > 65535) {
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
            toast("Failed to set websocket port" + ": " + e.toString());
            await refreshPorts();
        }
    };

    const handleKillPort = async () => {
        if (portToKill < 1 || portToKill > 65535) {
            toast.error("Invalid port number to kill");
            return;
        }

        try {
            await invoke("kill_process_on_port", { port: portToKill });
            toast.success(`Process on port ${portToKill} killed`);
        } catch (e: any) {
            toast("Failed to kill process" + ": " + e.toString());
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
                <Button
                    className="min-w-[12rem]"
                    onClick={updateAntPort}
                    disabled={isClientRunning}
                >
                    Set Ant port
                </Button>
                {isClientRunning && (
                    <Button onClick={() => showVersion("ant")}>
                        Show Version
                    </Button>
                )}
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
                <Button
                    className="min-w-[12rem]"
                    onClick={updateAnttpPort}
                    disabled={isClientRunning}
                >
                    Set AntTP port
                </Button>
                {isClientRunning && (
                    <Button onClick={() => showVersion("anttp")}>
                        Show Version
                    </Button>
                )}
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
                <Button
                    className="min-w-[12rem]"
                    onClick={updateDwebPort}
                    disabled={isClientRunning}
                >
                    Set DWeb port
                </Button>
                {isClientRunning && (
                    <Button onClick={() => showVersion("dweb")}>
                        Show Version
                    </Button>
                )}
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
                    className="min-w-[12rem]"
                    onClick={updateWebsocketPort}
                    disabled={isClientRunning}
                >
                    Set Websocket port
                </Button>
            </div>

            <div className="flex flex-row gap-2 items-center p-4 border-t mt-4 -mx-4">
                <input
                    type="number"
                    placeholder="Enter port"
                    value={portToKill}
                    onChange={(e) => setPortToKill(Number(e.target.value))}
                    className="border rounded p-1 w-32"
                    min={1}
                    max={65535}
                />
                <Button onClick={handleKillPort}>
                    Kill Process using port
                </Button>
            </div>
        </div>
    );
}
