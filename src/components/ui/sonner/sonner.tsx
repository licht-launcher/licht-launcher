import {
  CircleCheckIcon,
  InfoIcon,
  Loader2Icon,
  OctagonXIcon,
  TriangleAlertIcon,
} from "lucide-react";
import { Toaster as Sonner, type ToasterProps } from "sonner";

import { toasterStyles } from "./sonner.styles";

function Toaster({ theme = "dark", ...props }: ToasterProps) {
  return (
    <Sonner
      theme={theme}
      closeButton
      className={toasterStyles.root}
      icons={{
        success: <CircleCheckIcon className={toasterStyles.icon} />,
        info: <InfoIcon className={toasterStyles.icon} />,
        warning: <TriangleAlertIcon className={toasterStyles.icon} />,
        error: <OctagonXIcon className={toasterStyles.icon} />,
        loading: <Loader2Icon className={toasterStyles.loadingIcon} />,
      }}
      toastOptions={{
        classNames: {
          toast: toasterStyles.toast,
          closeButton: toasterStyles.closeButton,
        },
      }}
      {...props}
    />
  );
}

export { Toaster };
