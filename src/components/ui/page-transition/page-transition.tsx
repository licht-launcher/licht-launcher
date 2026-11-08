import { cn } from "cn";
import { createContext, type ReactNode, useContext, useState } from "react";

import { pageTransitionStyles } from "./page-transition.styles";

const PageTransitionContext = createContext(false);

export function PageTransition({
  children,
  animationKey,
  direction = "up",
}: {
  children: ReactNode;
  animationKey?: string | null;
  direction?: "up" | "left" | "right";
}) {
  const parentEntering = useContext(PageTransitionContext);
  const [animation, setAnimation] = useState({
    key: animationKey,
    alternate: false,
    parentEntering,
    enabled: !parentEntering,
    running: !parentEntering,
  });
  if (
    animation.key !== animationKey ||
    animation.parentEntering !== parentEntering
  ) {
    const keyChanged = animation.key !== animationKey;
    const enabled = !parentEntering && (keyChanged || animation.enabled);
    // Suppressed children stay still after the parent ends; only a new entry replays.
    setAnimation({
      key: animationKey,
      alternate: keyChanged ? !animation.alternate : animation.alternate,
      parentEntering,
      enabled,
      running: enabled && (keyChanged || animation.running),
    });
  }
  return (
    <PageTransitionContext.Provider value={parentEntering || animation.running}>
      <div
        data-slot="page-transition"
        data-direction={direction}
        className={cn(
          pageTransitionStyles.root,
          animation.enabled &&
            (animation.alternate
              ? pageTransitionStyles.enterAlternate
              : pageTransitionStyles.enter),
        )}
        onAnimationEnd={(event) => {
          const name = animation.alternate
            ? "page-enter-alternate"
            : "page-enter";
          if (
            event.target === event.currentTarget &&
            event.animationName === name
          ) {
            setAnimation((current) => ({ ...current, running: false }));
          }
        }}
      >
        {children}
      </div>
    </PageTransitionContext.Provider>
  );
}
