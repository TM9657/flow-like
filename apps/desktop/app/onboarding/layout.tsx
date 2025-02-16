import { Suspense } from "react";

export default function OnboardingLayout({
    children,
  }: Readonly<{
    children: React.ReactNode;
  }>){
    return <div className={"fixed top-0 left-0 right-0 bottom-0 bg-background z-50 flex flex-col justify-center items-center overflow-auto max-h-dvh max-w-[100dvw] p-4"}>
      <Suspense>
        {children}
      </Suspense>
    </div>
}