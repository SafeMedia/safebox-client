import { useEffect, useState } from "react";
import { Label } from "../ui/label";
import * as path from "@tauri-apps/api/path";
import { Button } from "../ui/button";
import { FolderSearchIcon } from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
import SubDividerLayout from "@/enums/sub-divider-layout";
import SubDivider from "./sub-divider";
import { useStorage } from "@/providers/storage-provider";
import { useTranslation } from "react-i18next";
import { downloadDir } from "@tauri-apps/api/path";

export default function StorageSettings() {
    const { t } = useTranslation();

    const { store } = useStorage();
    const [downloadFolder, setDownloadFolder] = useState("");
    const [isLoading, setIsLoading] = useState<boolean | null>(null);

    useEffect(() => {
        async function loadSettings() {
            if (!store) {
                setIsLoading(false);
                return;
            }

            try {
                const downloadFolder = await store.get<{ value: string }>(
                    "download-folder"
                );

                const defaultDownloadFolder = await downloadDir();

                if (!downloadFolder || !downloadFolder.value) {
                    setDownloadFolder(defaultDownloadFolder); // Use default path if none is set
                } else {
                    setDownloadFolder(downloadFolder.value); // Set stored folder if it exists
                }
            } catch (err) {
                console.error("Failed to load settings", err);
            } finally {
                setIsLoading(false);
            }
        }

        loadSettings(); // Load settings when store is initialized
    }, [store]); // Run when the store is set

    // directory browser to set new download location
    const handleBrowseClick = async () => {
        const selectedFolder = await open({
            multiple: false,
            directory: true,
        });

        if (selectedFolder) {
            setDownloadFolder(selectedFolder);
        }
    };

    return (
        <div className="items-center">
            <SubDivider title="Download" layout={SubDividerLayout.TOP} />
            {isLoading !== null && !isLoading && (
                <div className="p-4">
                    <Label htmlFor="location">{t("downloadLocation")}</Label>
                    <div className="flex">
                        <button
                            type="button"
                            onClick={async () => {
                                await handleBrowseClick();
                            }}
                            className="hover:bg-accent hover:text-accent-foreground  py-2 px-4 border border-input inline-flex items-center justify-center whitespace-nowrap rounded-l-md text-sm font-medium ring-offset-background transition-colors focus-visible:outline-none disabled:pointer-events-none disabled:opacity-50"
                        >
                            <FolderSearchIcon size={20} className="mr-2" />
                            {t("select")}
                        </button>

                        {/* Right-side file input */}
                        <input
                            id="input"
                            type="text"
                            className="block w-full rounded-r-md border border-input bg-background px-3 py-2 text-sm ring-offset-background file:border-0 file:bg-transparent file:text-sm file:font-medium file:text-foreground placeholder:text-muted-foreground focus-visible:outline-none  disabled:cursor-not-allowed disabled:opacity-50"
                            placeholder={t("selectDownloadFolder")}
                            value={downloadFolder}
                            readOnly
                        />
                    </div>

                    <div className="flex justify-end py-2">
                        <Button
                            variant={"outline"}
                            size={"sm"}
                            onClick={async () => {
                                const defaultDownloadsPath =
                                    await path.downloadDir();
                                setDownloadFolder(defaultDownloadsPath);
                            }}
                            className="mr-2"
                        >
                            {t("default")}
                        </Button>

                        <Button
                            variant={"outline"}
                            size={"sm"}
                            onClick={async () => {
                                if (store) {
                                    await store.set("download-folder", {
                                        value: downloadFolder,
                                    });
                                    await store.save();
                                    toast(t("downloadFolderUpdated"), {
                                        description: t(
                                            "yourDownloadFolderLocationHasBeenUpdated"
                                        ),
                                    });
                                }
                            }}
                        >
                            {t("save")}
                        </Button>
                    </div>
                </div>
            )}
        </div>
    );
}
