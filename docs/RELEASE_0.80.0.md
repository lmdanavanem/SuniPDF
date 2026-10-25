# SuniPDF 0.80.0

Development candidate; publication and package qualification are pending.

## Rotated dimensioned rectangles

Place a rectangle with explicit real width, height and clockwise rotation.
The rotation uses the placed lower-left corner as its pivot. The preview and
saved result share the same corner construction; width and height follow the
rotated sides. Zero degrees preserves the existing placement behavior.

Page, local and named-region calibrations retain their current meaning. The
entire rotated polygon must fit its permitted page/region, and unrepresentable
coordinates are refused. Changing the angle invalidates the prepared preview
until the new result is reviewed.

The result remains a four-vertex area measurement. Rotation survives saving
and reopening through those vertices, with existing units, quantities, Undo/Redo
and explicit recalibration. This adds rotated creation, not permanent rectangle
constraints or a separate post-save angle editor.

The command line accepts `--rotation-degrees`; finite angles are normalized to
the range from zero inclusive to 360 exclusive. Non-finite inputs are refused.

## Qualification and limits

Source, platform, geometry and package results will be recorded after their
checks complete. This increment does not add exact analytic circles, independent
X/Y scales, physical survey accuracy or universal external-reader compatibility.

[Plan workspace](PLAN_WORKSPACE.md) · [Roadmap](ROADMAP.md).
