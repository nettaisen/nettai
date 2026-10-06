--!strict
-- CloudMan, the navi of EXE5's CloudMan chips (EXE5's actor object #0x15, 0x080BC304; EXE5's own, built on EXE6's navi
-- template), brought by their navi chip controller. He appears (a sound) and, 10 ticks on, sets a cloud (./cloud)
-- on each free panel behind each enemy combatant, as far back as the free panels go; with none set he leaves at
-- once. Else he waits 30 ticks, raises his arms (animation 3) 30, holds them (animation 4) 110, stands 90, and
-- leaves; the controller moves on.

local navi_chips = require("@exelib/navi_chips/navi")
local cloud = require("./cloud")

export type State = {
    -- The controller waiting for him (the flag it gave him, +0x54).
    controller: Object?,
    -- The chip's level (its Param1: 0, 4 for DS), which picks his palette.
    level: number,
}

local cloudman = {}

local SPRITE = asset.sprite("cloudman")
local SOUND_APPEAR = asset.sound("appear")
-- By the level (0x080BC37C): DS's 6 (SP's level is 0).
local PALETTES: { [number]: number } = { [0] = 0, 0, 0, 0, 6 }
-- A panel a cloud takes: any but one with these flags (as one word).
local TAKEN = 0x3F800000
-- How long he holds his arms up, and when (ticks left) he lowers them.
local HOLD, LOWER = 0xC8, 0x5A

-- 0x080BC574: the clouds, for each enemy combatant (the last listed first): on the panels behind it, one by one,
-- while each exists and is free; each faces the enemy. How many were set.
local function set_clouds(me: Object): number
    local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
    local enemies = battle.alive_actors(bit32.bxor(me.alliance, 1))
    local set = 0
    for i = #enemies, 1, -1 do
        local enemy = enemies[i]
        local back = -enemy.facing
        local flip = bit32.bxor(enemy.flip, 1)
        local n = 1
        while true do
            -- (Past the field's columns the game reads no panel, as a byte reads none.)
            local x = int.u8(enemy.panel_x + back * n)
            if not field.check(x, enemy.panel_y, 0, TAKEN) then
                break
            end
            cloud.spawn(me, x, enemy.panel_y, me.element, flip, damage)
            set += 1
            n += 1
        end
    end
    return set
end

-- Action 0 (0x080BC3AC): he appears (a sound); 10 ticks; then the clouds, and he waits, or, with none, leaves.
local function appear(me: Object)
    if me.phase_init == 0 then
        battle.play_sound(SOUND_APPEAR)
        me.visible = true
    end
    if navi_chips.wait_on_init(me, 0xA, true) then
        me:set_action(if set_clouds(me) ~= 0 then 4 else 0x10)
    end
end

-- Action 4 (0x080BC3F0): 30 ticks.
local function stand(me: Object)
    if navi_chips.wait_on_init(me, 0x1E, true) then
        me:set_action(8)
    end
end

-- Action 8 (0x080BC414): his arms rise (animation 3), 30 ticks.
local function raise(me: Object)
    if me.phase_init == 0 then
        me.anim = 3
    end
    if navi_chips.wait_on_init(me, 0x1E, true) then
        me:set_action(0xC)
    end
end

-- Action 0xC (0x080BC43C): his arms up (animation 4), 200 ticks (a halfword store), the last 90 standing.
local function hold(me: Object)
    if me.phase_init == 0 then
        me.anim = 4
    end
    local left = navi_chips.countdown_on_init(me, HOLD)
    if left == nil then
        return
    end
    if left == LOWER then
        me.anim = 0
    elseif left <= 0 then
        me:set_action(0x10)
    end
end

-- Action 0x10 (0x080BC472): he vanishes; 10 ticks; then the controller moves on, and he goes.
local function leave(me: Object)
    if me.phase_init == 0 then
        me.visible = false
    end
    if navi_chips.wait_on_init(me, 0xA, true) then
        navi_chips.leave(me, (me.state :: State).controller)
    end
end

local ACTIONS = { [0] = appear, [4] = stand, [8] = raise, [0xC] = hold, [0x10] = leave }

-- 0x080BC328: on his panel, on the ground; his sprite (animation 0, his shadow), his palette by the level, facing
-- his side's way.
local function init(me: Object)
    navi_chips.stand(me, SPRITE, "ground", true)
    me.sprite.palette = PALETTES[(me.state :: State).level] or 0
    me:set_lifecycle("update")
end

cloudman.kind = new.kind {
    id = "cloudman/navi",
    pool = "actor",
    state = { controller = "object", level = "u8" },
    -- 0x080BC304.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            local action = ACTIONS[me.action]
            if not action then
                error(string.format("CloudMan action %#x reads past his table (0x080BC398)", me.action))
            end
            action(me)
        else
            me:free()
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080BC4A6: CloudMan for `user`, reporting to `controller` (the summon table's entry 15), of a chip of `level`.
function cloudman.summon_at(level: number): (user: Object, controller: Object, spec: NaviChipSpec) -> Object?
    return function(user: Object, controller: Object, spec: NaviChipSpec): Object?
        local o = navi_chips.spawn(cloudman.kind, user, spec)
        if not o then
            return nil
        end
        local s = o.state :: State
        s.controller = controller
        s.level = level
        return o
    end
end

-- CloudMan's and CloudManSP's (their Param1s 0), CloudManDS's (4).
cloudman.summon = cloudman.summon_at(0)
cloudman.summon_sp = cloudman.summon_at(0)
cloudman.summon_dark = cloudman.summon_at(4)

return cloudman
