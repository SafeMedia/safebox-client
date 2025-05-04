import { useEffect, useRef } from "react";
import { toast } from "sonner";
import { listen } from "@tauri-apps/api/event";
import { download } from "@/backend/logic";
import { UploadPayload } from "@/types/upload-file-event";
import { useConnection } from "@/providers/connection-provider";
import { AccountUser } from "@/types/account-user";
import { ToastPayload } from "@/types/payloads";

export default function Dashboard() {
    const { account } = useConnection();

    const accountRef = useRef<AccountUser | null>(null); // create a ref

    // whenever account changes, keep the ref updated
    useEffect(() => {
        accountRef.current = account;
    }, [account]);

    useEffect(() => {
        const unlistenDownload = listen<string>(
            "download-file",
            async (event) => {
                toast("Download Request", {
                    description: event.payload,
                });

                try {
                    await download(event.payload, event.payload);
                } catch (error) {
                    toast("Download Failed", {
                        description: `Failed to download: ${event.payload}`,
                    });
                }
            }
        );

        const unlistenUpload = listen<UploadPayload>(
            "upload-file",
            async (event) => {
                if (!accountRef.current) {
                    toast("Not Signed In", {
                        description: `Upload requested but you are not signed in`,
                    });
                    return;
                }

                // assume event.payload already contains all the info needed
                const { name, success, error, xorname } = event.payload;

                if (success) {
                    toast(`File '${name}' Uploaded`, {
                        description: `${xorname}`,
                    });
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

        const unlistenToast = listen<ToastPayload>(
            "show-toast",
            async (event) => {
                const { title, description } = event.payload;

                toast(title, {
                    description,
                });
            }
        );

        return () => {
            unlistenDownload.then((fn) => fn());
            unlistenUpload.then((fn) => fn());
            unlistenToast.then((fn) => fn());
        };
    }, []);

    return (
        <div className="p-4 space-y-2">
            <div className="flex flex-row"></div>
        </div>
    );
}
