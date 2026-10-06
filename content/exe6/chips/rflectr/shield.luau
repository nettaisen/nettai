--!strict
-- A Reflector's shield (the original's attack object #0x2B, `sub_80C96A0`):
-- the barrier a navi holds up while it guards. Visual only: the guard is
-- the navi's own status. It follows its owner at the owner's attach point
-- 6, shows while its owner does, and ends with a short fade once the owner
-- clears the slot it was stored in (its first related object). It runs
-- while paused and dimmed, but doesn't animate while dimmed. See chips.md
-- §3.3. The behavior is EXE5's too (@exelib/rflectr/shield), made here with
-- EXE6's sound; its looks are its holders'.

local common = require("@exelib/rflectr/shield")

local shield = {}

-- A shield's look (`ReflectorShieldLook`, types.d.luau; a row of
-- `byte_80C9664`, by the original's second parameter), which its holder
-- defines: its sprite, its animation and fade's, the fade's ticks and its
-- palette.
export type Look = ReflectorShieldLook

shield.look = common.look

-- The Reflector shield's sheet (the chips' and the NaviCust programs'
-- looks are animations of it).
shield.sprite = asset.sprite("reflector-shield")

local made = common.make { id = "rflectr/shield", sound = asset.sound("follow-effect") }
shield.kind = made.kind

-- `sub_80C97E0`: a shield of `look` for `owner`, `offset` from it (its
-- attach point, in the spawn position); stored in the owner's first
-- related object, unless the pool is full.
shield.spawn = made.spawn

return shield
