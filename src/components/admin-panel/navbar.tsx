import { SheetMenu } from "@/components/admin-panel/sheet-menu";
import { ThemeToggler } from "../theme-toggler";

export function Navbar() {
    return (
        <header className="sticky top-0 z-40 w-full bg-background">
            <div className="mx-4 sm:mx-8 flex h-14 items-center">
                <div className="flex items-center space-x-4 lg:space-x-0">
                    <SheetMenu />
                </div>
                <div className="flex flex-1 items-center justify-end gap-4">
                    <ThemeToggler />
                </div>
            </div>
        </header>
    );
}
