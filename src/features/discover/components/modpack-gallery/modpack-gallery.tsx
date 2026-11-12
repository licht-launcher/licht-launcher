import type { ModpackImage } from "@/lib/generated/ModpackImage";

import { modpackGalleryStyles } from "./modpack-gallery.styles";

/** The pack's screenshots, each with its title as caption when it has one. */
export function ModpackGallery({
  images,
}: {
  images: readonly ModpackImage[];
}) {
  return (
    <ul aria-label="Gallery" className={modpackGalleryStyles.grid}>
      {images.map((image) => (
        <li key={image.url}>
          <figure className={modpackGalleryStyles.figure}>
            <img
              src={image.url}
              alt={image.title ?? ""}
              loading="lazy"
              className={modpackGalleryStyles.image}
            />
            {image.title && (
              <figcaption className={modpackGalleryStyles.caption}>
                {image.title}
              </figcaption>
            )}
          </figure>
        </li>
      ))}
    </ul>
  );
}
