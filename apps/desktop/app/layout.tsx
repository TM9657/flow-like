"use client";
import {
  QueryClient
} from '@tanstack/react-query';
import { PersistQueryClientProvider } from "@tanstack/react-query-persist-client";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { Bit } from "@tm9657/flow-like-ui";
import { ThemeProvider } from "@tm9657/flow-like-ui/components/theme-provider";
import { Toaster } from "@tm9657/flow-like-ui/components/ui/sonner";
import { TooltipProvider } from "@tm9657/flow-like-ui/components/ui/tooltip";
import "@tm9657/flow-like-ui/globals.css";
import { createIDBPersister } from "@tm9657/flow-like-ui/lib/persister";
import { ReactFlowProvider } from "@xyflow/react";
import { useTheme } from "next-themes";
import { Inter } from "next/font/google";
import { Suspense, useEffect } from "react";
import { toast } from "sonner";
import { AppSidebar } from '../components/app-sidebar';
import PostHogPageView from "./PostHogPageView";
import { PHProvider } from "./provider";

const inter = Inter({ subsets: ["latin"] });

const persister = createIDBPersister();
const queryClient = new QueryClient()

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  const { resolvedTheme, theme } = useTheme()

  useEffect(() => {
    console.log(resolvedTheme)
  }, [resolvedTheme])

  useEffect(() => {
    console.log(theme)
  }, [theme])

  async function initDownloads() {
    const downloads: any = await invoke("init_downloads");
    console.dir({ downloads })
    const items = Object.keys(downloads).map((bitId) => {
      const bit: Bit = downloads[bitId];
      return bit
    })

    const download_requests = items.map((item) => {
      return invoke("resume_download", { bit: item })
    })

    await Promise.allSettled([...download_requests])
  }


  useEffect(() => {
    initDownloads()

    let subscriptions: (Promise<UnlistenFn> | undefined)[] = []
    const unlistenFn = listen("toast", (event: { payload: { message: string, level: "success" | "error" | "info" | "warning" }[] }) => {
      for (const message of event.payload) {
        if (message.level === "success") return toast.success(message.message)
        if (message.level === "error") return toast.error(message.message)
        if (message.level === "warning") return toast.warning(message.message)
        toast.info(message.message)
      }
    })
    subscriptions.push(unlistenFn)

    return () => {
      (async () => {
        for await (const subscription of subscriptions) {
          if (subscription) (subscription)()
        }
      })()
    }
  }, []);

  return (
    <html lang="en" suppressHydrationWarning suppressContentEditableWarning>
      <PHProvider>
        <ReactFlowProvider>
          <PersistQueryClientProvider
            client={queryClient}
            persistOptions={{
              persister
            }}
          >
            <TooltipProvider>
              <Toaster />
              <body className={inter.className}>
                <Suspense fallback={<div className="flex flex-1 justify-center items-center">{"Loading..."}</div>}>
                  <PostHogPageView />
                  <ThemeProvider
                    attribute="class"
                    defaultTheme="system"
                    enableSystem
                    disableTransitionOnChange
                  >
                    <AppSidebar>
                      {children}
                    </AppSidebar>
                  </ThemeProvider>
                </Suspense>
              </body>
            </TooltipProvider>
          </PersistQueryClientProvider>
        </ReactFlowProvider>
      </PHProvider>
    </html>
  );
}