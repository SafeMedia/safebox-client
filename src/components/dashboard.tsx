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

    useEffect(() => {
        // Listen for download-file events
        const unlistenDownload = listen<string>(
            "download-file",
            async (event) => {
                toast("Download Request", {
                    description: event.payload,
                });
                try {
                    await download(event.payload, event.payload);
                } catch {
                    toast("Download Failed", {
                        description: `Failed to download: ${event.payload}`,
                    });
                }
            }
        );

        // Listen for upload-file events
        const unlistenUpload = listen<UploadPayload>(
            "upload-file",
            async (event) => {
                if (!accountRef.current) {
                    toast("Not Signed In", {
                        description: `Upload requested but you are not signed in`,
                    });
                    return;
                }
                const { name, success, error, xorname } = event.payload;

                if (success) {
                    toast(`File '${name}' Uploaded`, { description: xorname });
                } else if (error) {
                    toast("Upload Failed", {
                        description: "Something went wrong during upload.",
                    });
                } else {
                    toast("Unknown Upload Status", {
                        description: `Could not determine result for "${name}"`,
                    });
                }
            }
        );

        // Listen for generic toast events
        const unlistenToast = listen<ToastPayload>("show-toast", (event) => {
            const { title, description } = event.payload;
            toast(title, { description });
        });

        // Cleanup listeners on unmount
        return () => {
            unlistenDownload.then((fn) => fn());
            unlistenUpload.then((fn) => fn());
            unlistenToast.then((fn) => fn());
        };
    }, []);

    // Toggles the client on/off by invoking Rust commands
    const toggleClient = async () => {
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

    return (
        <div className="p-4 space-y-2">
            <div className="flex flex-row gap-2 items-center">
                <Button onClick={toggleClient}>
                    {isClientRunning ? "Stop Server" : "Start Server"}
                </Button>
            </div>
        </div>
    );
}
