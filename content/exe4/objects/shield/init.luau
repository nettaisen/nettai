--!strict
-- EXE4's guard shield (attack object #0x2B, 0x080D2224; EXE6's Reflectors' `sub_80C96A0`, @exelib/rflectr/shield):
-- the shield a navi holds up while it guards. Visual only: the guard is the navi's own status. It follows its owner
-- at the offset it was spawned with, shown as its owner is, and fades once the owner clears the slot it was stored in
-- (its first related object). EXE4's look: one sheet (0c-1b), its look by its row (animation 2 x the row, the fade the
-- next, 14 ticks for every row), its sound 0xC1; it runs while dimmed (not while paused) and animates then too, with
-- no test of its owner's having vanished, and its spawn (0x080D2338) takes its owner's side alone.

local common = require("@exelib/rflectr/shield")

local shield = {}

local SPRITE = asset.sprite("reflector-shield")
local FADE_TICKS = 14

local made = common.make {
    id = "guard-shield",
    sound = asset.sound("shield"),
    animates_while_dimmed = true,
    runs_while_paused = false,
    hides_with_vanished_owner = false,
    side_only = true,
}

shield.kind = made.kind

-- The looks by row (its second parameter): 1 the Reflect program's, 2 the Shield program's (0x080D2248: animation
-- 2 x the row; 0x080D230A: the fade the next).
local LOOKS: { [number]: ReflectorShieldLook } = {}
for row = 1, 2 do
    LOOKS[row] = common.look { sprite = SPRITE, anim = 2 * row, palette = 0, fade_anim = 2 * row + 1, fade_ticks = FADE_TICKS }
end

-- 0x080D2338: a shield of look row `row` for `owner`, `offset` from it (in the spawn position); stored in the
-- owner's first related object, unless the pool is full; it runs while dimmed.
function shield.spawn(owner: Object, offset: Vec3, row: number): Object?
    local look = LOOKS[row]
    if not look then
        error(string.format("guard shield row %d has no look", row))
    end
    return made.spawn(owner, offset, look)
end

return shield
