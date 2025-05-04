export function Footer() {
    return (
        <div className="relative w-full bg-background h-16 border-t border-primary-foreground">
            <div className="mx-4 md:mx-8 flex h-full items-center justify-between relative">
                <div className="text-xs md:text-sm leading-loose text-muted-foreground">
                    Powered by{" "}
                    <a
                        href="https://autonomi.com"
                        target="_blank"
                        rel="noopener noreferrer"
                        className="font-medium underline underline-offset-4"
                    >
                        Autonomi
                    </a>
                    .
                </div>
            </div>
        </div>
    );
}
