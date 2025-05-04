import { ReactNode } from "react";
import AdminPanelLayout from "@/components/admin-panel/admin-panel-layout";
import "../index.css";
import { ContentLayout } from "./admin-panel/content-layout";
import { ThemeProvider } from "@/providers/theme-provider";
import "@/i18n/config";
import { LanguageProvider } from "@/providers/language-provider";
import { useConnection } from "@/providers/connection-provider";
import DisconnectedPanel from "./disconnected-panel";

interface RootLayoutProps {
    children: ReactNode; // explicitly type children as ReactNode
}

const RootLayout = ({ children }: RootLayoutProps) => {
    const { isConnected } = useConnection();

    return (
        <div className={"antialiased flex flex-col min-h-screen"}>
            {isConnected ? (
                <ThemeProvider
                    attribute="class"
                    defaultTheme="system"
                    disableTransitionOnChange
                >
                    <LanguageProvider>
                        <AdminPanelLayout>
                            <ContentLayout>
                                <main className="flex-grow">{children}</main>
                            </ContentLayout>
                        </AdminPanelLayout>
                    </LanguageProvider>
                </ThemeProvider>
            ) : (
                <DisconnectedPanel />
            )}
        </div>
    );
};

export default RootLayout;
