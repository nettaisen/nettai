#!/usr/bin/env python3
"""nettai's AltStore source, for a release's IPA: altstore.json beside this script (what the source and the app
say of themselves) with the release's version and the app's permissions filled in from the IPA, on stdout.

    packaging/ios/altstore.py <nettai-ios.ipa> <the IPA's download URL> > altstore.json

The version is the IPA's Info.plist's (CFBundleShortVersionString, and CFBundleVersion as the build), which AltStore
matches against the app it downloads; so is the oldest iOS it runs on, and its privacy permissions (each
NS...UsageDescription), which AltStore refuses to install an app without the source declaring. The IPA is unsigned,
so it carries no entitlements: the sideloading tool's signature adds its own.
"""

import datetime
import hashlib
import json
import plistlib
import sys
import zipfile
from pathlib import Path


def main() -> None:
    if len(sys.argv) != 3:
        sys.exit(f"usage: {sys.argv[0]} <nettai-ios.ipa> <download URL>")
    ipa, url = Path(sys.argv[1]), sys.argv[2]
    with zipfile.ZipFile(ipa) as bundle:
        info = plistlib.loads(bundle.read("Payload/nettai.app/Info.plist"))
    data = ipa.read_bytes()

    source = json.loads(Path(__file__).with_name("altstore.json").read_text())
    app = source["apps"][0]
    if app["bundleIdentifier"] != info["CFBundleIdentifier"]:
        sys.exit(f"the IPA is {info['CFBundleIdentifier']}, the source's app {app['bundleIdentifier']}")
    app["versions"] = [
        {
            "version": info["CFBundleShortVersionString"],
            "buildVersion": info["CFBundleVersion"],
            "date": datetime.datetime.now(datetime.UTC).strftime("%Y-%m-%dT%H:%M:%SZ"),
            "downloadURL": url,
            "size": len(data),
            "sha256": hashlib.sha256(data).hexdigest(),
            "minOSVersion": info["MinimumOSVersion"],
        }
    ]
    app["appPermissions"] = {
        "entitlements": [],
        "privacy": {key: value for key, value in sorted(info.items()) if key.endswith("UsageDescription")},
    }
    json.dump(source, sys.stdout, indent=2, ensure_ascii=False)
    print()


if __name__ == "__main__":
    main()
