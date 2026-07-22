# Check Modeled Device Consent

This recipe runs the modeled consent path without hardware. It reports that a
pose sensor read fails when either kernel authority or visible session consent
is absent, then shows that the retention reaper removes a sample and its only
referenced content after the window closes.
