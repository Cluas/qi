import { Haptics, isAvailable as isMobileHost } from "nsh/mobile";

/**
 * Safe CoreHaptics vocabulary for iOS & mobile runtimes.
 * Silently no-ops on macOS desktop / headless test environments.
 */
export function hapticSelection(): void {
  try {
    if (isMobileHost) {
      Haptics.selection();
    }
  } catch {
    // Non-mobile hosts or simulators without haptic engines safely ignore
  }
}

export function hapticSoftImpact(): void {
  try {
    if (isMobileHost) {
      Haptics.impact("soft");
    }
  } catch {
    // Ignore on unsupported hosts
  }
}

export function hapticLightImpact(): void {
  try {
    if (isMobileHost) {
      Haptics.impact("light");
    }
  } catch {
    // Ignore on unsupported hosts
  }
}

export function hapticStampImpact(): void {
  try {
    if (isMobileHost) {
      Haptics.impact("medium");
    }
  } catch {
    // Ignore on unsupported hosts
  }
}

export function hapticSuccess(): void {
  try {
    if (isMobileHost) {
      Haptics.notification("success");
    }
  } catch {
    // Ignore on unsupported hosts
  }
}
