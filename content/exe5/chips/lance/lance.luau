--!strict
-- EXE5's lance (attack object #0x4F, EXE6's #0x6F): EXE6's (@exelib/lance/lance, the same code) but for its sprite
-- (0C-2B), sound, and the chips' hits (0x080CF904 by their Param1, each chip's own: Lance's ./init, DrkLance's).

local lance = require("@exelib/lance")

return {
    kind = lance.make_kind("lance", {
        sprite = asset.sprite("lance"),
        sound = asset.sound("sword-swing"),
    }),
}
