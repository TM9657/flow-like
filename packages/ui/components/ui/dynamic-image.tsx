import { useEffect, useRef } from "react";

export function DynamicImage({url, className}: Readonly<{url: string, className: string}>) {
    const ref = useRef<HTMLDivElement>(null)
    
    useEffect(() => {
        if(!ref.current) return

        ref.current.style.maskImage = `url(${url})`
        ref.current.style.maskSize = `contain`      
        ref.current.style.maskRepeat = "no-repeat"
    }, [url])

    if(!url.includes(".svg")) return <img alt="dynamic_icon" src={url} className={`border-0 ${className}`}/>

    return <div ref={ref} className={`border-0 ${className}`}/>
}