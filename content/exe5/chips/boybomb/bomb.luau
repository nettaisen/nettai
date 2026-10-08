--!strict
-- EXE5's BoyBomb1-3 (the dimming handler's subtype 58): EXE6's leftover code (EXE6's effect object #0x67, `sub_80E7DC4`,
-- and attack object #0x3E, `sub_80CBB44`, which no EXE6 chip spawns) with EXE5's numbers. Its controller (effect object
-- #0x67, 0x080E8008: the usual dimming phases) sets a bomb on the panel ahead of the user if it is free, and waits 30
-- ticks. The bomb (attack object #0x3E, 0x080CCDF4) is @exelib/boybomb/bomb with EXE5's look: it fades in, counts down
-- 60 ticks blinking and blows up over the 3x3 around it; any one side's hit pushes it a panel away (EXE5's reaction by
-- the hitters' flips, 0x08017A78); EXE5 stores the framework's 8, its standing, as 6 (`obstacle.action_byte`).

local collision = require("../../rules/collision")
local common = require("@exelib/boybomb/bomb")
local dimming_chips = require("@exelib/dimming")

local bomb = {}

export type ControllerState = {
    -- The Atk+ bonus (+0x32): it adds to the bomb's damage word.
    bonus: number,
}

local WAIT = 30
-- Where the controller sets it: solid with nothing on it.
local SOLID, OCCUPIED = 0x10, 0x0F880080

-- The bomb (attack object #0x3E, 0x080CCDF4): sprite 10-36; NameID 0xD4 (actor record 0xD4, 0x08014C94: version 1,
-- AI index 0x21); its collision (Param1 1) a neutral body (row 0x0E) that everything reaches (row 0x0F), its blast an
-- attack at navis; EXE5's effects (0x080DFC94): row 6 (0C-12) as it comes, row 0 (14-00) over the blast, row 0x14
-- (14-0D, animation 2) when removed, row 0x0E (14-01) when broken; where it may stand by its side (0x080CD2BC):
-- without its side's navi or another body; any one side's hit pushes it a panel away (0x08017A78, by the hitters'
-- flips); the obstacle rows, flinch, frozen and bubbled among them.
local BOMB = common.make {
    id = "boybomb/bomb",
    sprite = asset.sprite("boy-bomb"),
    identity = new.identity { class = "field_object", version = 1, ai_index = 0x21 },
    body = { is = collision.neutral_body, hits = collision.everything },
    blast = {
        is = collision.attack,
        hits = collision.hits_navis,
        effect = new.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 0 },
        sound = asset.sound("target-move"),
    },
    comes = new.effect { sprite = asset.sprite("heal"), anim = 0, palette = 0 },
    -- (0x112: 0x080CCE82 loads it as 19 + 255.)
    place_sound = asset.sound("place"),
    broken = new.effect { sprite = asset.sprite("impact"), anim = 0, palette = 0 },
    break_sound = asset.sound("hit-bomb-1"),
    removed = new.effect { sprite = asset.sprite("puff"), anim = 2, palette = 0 },
    stand_forbid = { [0] = 0x0B800000, [1] = 0x06800000 },
    push = "any_hit",
    actions = {
        [1] = obstacle.return_to_idle,
        [3] = obstacle.flinch,
        [4] = obstacle.paralyzed,
        [5] = obstacle.knocked_back,
        [6] = obstacle.frozen,
        [7] = obstacle.bubbled,
    },
    reserves_panel = true,
    releases_tracking = true,
}
bomb.kind = BOMB.kind

-- 0x080E804C: the bomb on the panel ahead if it is free (0x080E80A8), with the controller's damage word and bonus;
-- then 30 ticks.
local function effect(me: Object)
    if me.phase_init == 0 then
        local x, y = me.panel_x + me.facing, me.panel_y
        if field.check(x, y, SOLID, OCCUPIED) then
            BOMB.spawn(me, x, y, me.element, dimming_chips.damage_plus(me, (me.state :: ControllerState).bonus))
        end
        me.timer = WAIT
        me.phase_init = 4
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left == 0 then
        dimming_chips.done(me)
    end
end

bomb.controller = new.kind {
    id = "boybomb/controller",
    pool = "effect",
    state = { bonus = "u16" },
    -- 0x080E8008.
    update = dimming_chips.phases { name = "BoyBomb", effect = effect },
}

-- 0x080E8086: the chips' `dimming` hook, the controller with the dimming's bonus. Its position is the dimming
-- handler's registers: the user's row, the element and the hook's own address (the handler's `bx r3`, Thumb).
-- (The address is Team ProtoMan US's ROM's; compat/games.toml has the other ROMs'.)
local HOOK_ADDRESS = 0x080E8087

function bomb.hook(user: Object, spec: DimmingChipSpec): Object?
    local o = dimming_chips.spawn(bomb.controller, user, spec)
    if o then
        o.pos = Vec3.new(user.panel_y, spec.element, HOOK_ADDRESS);
        (o.state :: ControllerState).bonus = spec.bonus
    end
    return o
end

return bomb
