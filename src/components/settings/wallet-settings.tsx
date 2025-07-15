import { useEffect, useState } from "react";
import { Label } from "../ui/label";
import { Button } from "../ui/button";
import { toast } from "react-toastify";
import { useStorage } from "@/providers/storage-provider";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";

export default function WalletSettings() {
    const { t } = useTranslation();
    const { store } = useStorage();
    const [privateKey, setPrivateKey] = useState("");
    const [password, setPassword] = useState("");
    const [isClientRunning, setIsClientRunning] = useState(false);

    const checkServerRunning = async () => {
        try {
            const running = await invoke<boolean>("is_server_running");
            setIsClientRunning(running);
        } catch {
            setIsClientRunning(false);
        }
    };

    useEffect(() => {
        async function loadWallet() {
            if (!store) return;

            const storedKey = await store.get<{ value: string }>(
                "wallet-private-key"
            );
            const storedPassword = await store.get<{ value: string }>(
                "wallet-password"
            );

            if (storedKey?.value) setPrivateKey(storedKey.value);
            if (storedPassword?.value) setPassword(storedPassword.value);
        }

        loadWallet();
        checkServerRunning(); // check server status on mount
    }, [store]);

    const saveWallet = async () => {
        if (!store) return;
        if (!privateKey.trim()) {
            toast.error("Private Key Required");
            return;
        }

        await store.set("wallet-private-key", { value: privateKey.trim() });
        await store.set("wallet-password", { value: password.trim() });
        await store.save();
        toast.success("Wallet Imported");

        try {
            await invoke("import_wallet");
        } catch (error) {
            toast.error("Wallet Import Failed");
            console.error("Wallet import failed:", error);
        }
    };

    return (
        <div className="p-4">
            <Label>{"Wallet Private Key"}</Label>
            <input
                type="password"
                value={privateKey}
                onChange={(e) => setPrivateKey(e.target.value)}
                placeholder={"Enter Wallet Private Key"}
                className="w-full border px-3 py-2 rounded mb-4"
            />

            <Label>
                {"Wallet Password"} ({"Optional"})
            </Label>
            <input
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                placeholder={"Enter Password"}
                className="w-full border px-3 py-2 rounded mb-4"
            />

            <div className="flex justify-end">
                <Button onClick={saveWallet} disabled={isClientRunning}>
                    {t("save")}
                </Button>
            </div>
            <div className="flex justify-end">
                {isClientRunning && (
                    <p className="text-red-500 mt-2">
                        {"Cannot import wallet while server is running"}
                    </p>
                )}
            </div>
        </div>
    );
}
