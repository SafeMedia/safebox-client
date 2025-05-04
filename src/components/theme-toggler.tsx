import { MoonIcon, SunIcon } from "@radix-ui/react-icons";
import { useTheme } from "next-themes";
import { Switch } from "@headlessui/react";
import { useEffect, useState } from "react";
import { load, Store } from "@tauri-apps/plugin-store";
import { useTranslation } from "react-i18next";

export function ThemeToggler() {
    const { t } = useTranslation();
    const { theme, setTheme, resolvedTheme } = useTheme();
    const [enabled, setEnabled] = useState(false); // Switch state
    const [isLoading, setIsLoading] = useState(true); // Loading state
    const [store, setStore] = useState<Store | null>(null);

    // initialize the store when the component mounts
    useEffect(() => {
        const initializeStore = async () => {
            try {
                const storeInstance = await load("store.bin", {
                    autoSave: true,
                });
                setStore(storeInstance); // set the store instance
                await checkInitialTheme(storeInstance); // call the check initial theme with the store instance
            } catch (error) {
                console.error("Failed to initialize store:", error);
                setIsLoading(false);
            }
        };

        initializeStore();
    }, []);

    // function to check the initial theme from the store or system
    const checkInitialTheme = async (store: Store) => {
        if (!store) {
            setIsLoading(false); // ensure loading is false if store is not initialized
            return;
        }

        try {
            const savedTheme = await store.get<{ value: string }>("theme");

            if (!savedTheme) {
                setTheme("light");
                await store.set("theme", { value: "light" });
                setEnabled(false);
            } else if (
                savedTheme.value === "dark" ||
                savedTheme.value === "light"
            ) {
                setTheme(savedTheme.value);
                setEnabled(savedTheme.value === "dark");
            }
        } catch (error) {
            setTheme("light");
            setEnabled(false);
        } finally {
            setIsLoading(false); // loading is complete
        }
    };

    // sync the switch toggle state with the current theme or resolvedTheme
    useEffect(() => {
        if (!isLoading) {
            setEnabled(
                theme === "dark" ||
                    (theme === "system" && resolvedTheme === "dark")
            );
        }
    }, [theme, resolvedTheme, isLoading]);

    // handler to toggle the theme
    const handleThemeToggle = async () => {
        const newTheme = enabled ? "light" : "dark"; // toggle theme
        setTheme(newTheme); // Set new theme
        await store?.set("theme", { value: newTheme }); // save new theme in store
        await store?.save(); // ensure changes are persisted
        setEnabled(!enabled); // update switch state
    };

    // show blank while loading to prevent flicker
    if (isLoading) {
        return <div></div>; // this can be replaced with a spinner or placeholder if required.
    }

    // return the theme toggle switch once the app has loaded
    return (
        <Switch
            checked={enabled}
            onChange={handleThemeToggle} // toggle theme on change
            className={`${enabled ? "bg-secondary" : "bg-border"}
          group relative inline-flex h-6 w-11 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none`}
        >
            <span className="sr-only">{t("useSetting")}</span>
            <span className="pointer-events-none relative inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out group-data-[checked]:translate-x-5">
                <span
                    aria-hidden="true"
                    className="absolute inset-0 flex h-full w-full items-center justify-center transition-opacity duration-200 ease-in group-data-[checked]:opacity-0 group-data-[checked]:duration-100 group-data-[checked]:ease-out"
                >
                    <SunIcon color="black" />
                </span>
                <span
                    aria-hidden="true"
                    className="absolute inset-0 flex h-full w-full items-center justify-center opacity-0 transition-opacity duration-100 ease-out group-data-[checked]:opacity-100 group-data-[checked]:duration-200 group-data-[checked]:ease-in"
                >
                    <MoonIcon color="black" />
                </span>
            </span>
        </Switch>
    );
}
