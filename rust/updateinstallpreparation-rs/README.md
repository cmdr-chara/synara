# updateinstallpreparation-rs

The source returns a coordinator object holding four closures over private
state. Closures do not cross the wire; the recorder serializes each as a
`{"$function": "<name>"}` marker, so the port reproduces that shape.

napi-rs bridge for `apps/desktop/src/updateInstallPreparation.ts`.

## Functions

- `makeUpdateInstallPreparationCoordinator` (`apps/desktop/src/updateInstallPreparation.ts:10`)