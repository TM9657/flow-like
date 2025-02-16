import type { IDate } from "../types";

export function formatRelativeTime(date: IDate) {
    const diffMilliseconds = Date.now() - date.secs_since_epoch * 1000;
  
    console.dir({
        diffMilliseconds,
        date,
        now: Date.now()
    })

    const seconds = Math.round(diffMilliseconds / 1000);
    const minutes = Math.round(seconds / 60);
    const hours = Math.round(minutes / 60);
    const days = Math.round(hours / 24);
  
    // Internationalization (i18n)
    const formatter = new Intl.RelativeTimeFormat(undefined, { 
      numeric: 'auto', // Use numbers when appropriate
      style: 'long'    // More descriptive, e.g., "2 days ago" 
    });
  
    if (seconds < 60) {
      return formatter.format(-1 * seconds, 'second'); 
    } else if (minutes < 60) {
      return formatter.format(-1 * minutes, 'minute');
    } else if (hours < 24) {
      return formatter.format(-1 * hours, 'hour');
    } else {
      return formatter.format(-1 * days, 'day');
    }
  }

  export function parseTimespan(start: IDate, end: IDate) {
    if (start.nanos_since_epoch > end.nanos_since_epoch) {
      const old_end = end
      end = start
      start = old_end
    }
    
    const diff = end.nanos_since_epoch - start.nanos_since_epoch
    const μs = diff / 1000

    if(μs < 1000) return `${μs.toFixed(2)}μs`
    const ms = μs / 1000
    if(ms < 1000) return `${ms.toFixed(2)}ms`
    const s = ms / 1000
    if(s < 60) return `${s.toFixed(2)}s`
    const m = s / 60
    if(m < 60) return `${m.toFixed(2)}m`
    const h = m / 60
    if(h < 24) return `${h.toFixed(2)}h`
    const d = h / 24
    return `${d.toFixed(2)}d`
}