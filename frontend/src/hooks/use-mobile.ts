import * as React from "react"

const MOBILE_BREAKPOINT = 768
const QUERY = `(max-width: ${MOBILE_BREAKPOINT - 1}px)`

// Subscribing to matchMedia is exactly what useSyncExternalStore is for: it reads
// an external system without a setState-in-effect round trip (react-hooks/set-state-in-effect),
// and it returns the correct value on the first render instead of undefined.
function subscribe(onChange: () => void) {
  const mql = window.matchMedia(QUERY)
  mql.addEventListener("change", onChange)
  return () => mql.removeEventListener("change", onChange)
}

export function useIsMobile() {
  return React.useSyncExternalStore(
    subscribe,
    () => window.matchMedia(QUERY).matches,
    () => false, // server/prerender snapshot
  )
}
