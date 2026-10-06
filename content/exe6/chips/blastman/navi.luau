--!strict
-- BlastMan, the navi of the BlastMan chips (the original's actor object
-- #0x06, `sub_80B8EA0`), brought by their navi chip controller. He
-- appears, winds up, and sends a fire blast across the field along his row
-- and the rows above and below it (./fire), from behind his side's edge.
-- Then he leaves, and the controller moves on. See docs/engine/chips.md
-- §3.6.14.

local fire = require("./fire")
local navi_chips = require("../../lib/navi_chips/navi")

export type State = {
    -- The controller waiting for him (its pointer sits in his collision
    -- data slot).
    controller: Object?,
}

local blastman = {}

local SPRITE = asset.sprite("blast-fire")
local SOUND_APPEAR = asset.sound("appear")
local SOUND_BLAST_MAN = asset.sound("blast-man")

-- His blasts' speed (16.16 a tick; the original's Param1 4, a row of
-- `byte_80C8454`).
local BLAST_SPEED = 0x6_0000

-- `sub_80B8EC4`: on his panel, on the ground.
local function init(me: Object)
    navi_chips.stand(me, SPRITE, "ground", true)
    me:set_lifecycle("update")
end

-- Enter the phase (phase 4, a word store of the timers when `word`), or
-- count down; true once the timer is 0 or below (or, `past_zero`, below
-- 0).
local function countdown(me: Object, ticks: number, word: boolean, past_zero: boolean): boolean
    if me.phase == 0 then
        me.phase = 4
        me.timer = ticks
        if word then
            me.timer2 = 0
        end
        return false
    end
    local left = me.timer - 1
    me.timer = left
    return if past_zero then left < 0 else left <= 0
end

-- `sub_80B8F30`: he appears; 3 ticks.
local function appear(me: Object)
    if me.phase == 0 then
        me.anim = 3
        battle.play_sound(SOUND_APPEAR)
        me.visible = true
    end
    if countdown(me, 3, true, false) then
        me:set_action(4)
    end
end

-- `sub_80B8F66`: 30 ticks standing.
local function stand(me: Object)
    if me.phase == 0 then
        me.anim = 0
    end
    if countdown(me, 0x1E, true, false) then
        me:set_action(8)
    end
end

-- `sub_80B8F8E`: 20 ticks winding up.
local function wind_up(me: Object)
    if me.phase == 0 then
        me.anim = 0xA
    end
    if countdown(me, 0x14, false, false) then
        me:set_action(0xC)
    end
end

-- `sub_80B903A`: a fire blast along each valid row of his three, from
-- behind his side's edge toward the other.
local function blast(me: Object)
    local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
    for dy = -1, 1 do
        local y = me.panel_y + dy
        if field.valid(me.panel_x, y) then
            local x, dir: FireBlastDirection = 0, "right"
            if me.alliance ~= 0 then
                x, dir = 7, "left"
            end
            fire.spawn(me, x, y, me.element, { speed = BLAST_SPEED, direction = dir, while_dimmed = true }, damage)
        end
    end
end

-- `sub_80B8FB6`: the blasts; 61 ticks (to -1).
local function attack(me: Object)
    if me.phase == 0 then
        me.anim = 0xB
        blast(me)
        battle.play_sound(SOUND_BLAST_MAN)
    end
    if countdown(me, 0x3C, false, true) then
        me:set_action(0x10)
    end
end

-- `sub_80B8FEA`: 4 ticks (to -1), then he goes and the controller moves
-- on.
local function leave(me: Object)
    if me.phase == 0 then
        me.anim = 4
    end
    if not countdown(me, 3, true, true) then
        return
    end
    navi_chips.leave(me, (me.state :: State).controller)
end

local ACTIONS = { [0] = appear, [4] = stand, [8] = wind_up, [0xC] = attack, [0x10] = leave }

blastman.kind = new.kind {
    id = "blastman/navi",
    pool = "actor",
    state = { controller = "object" },
    -- `sub_80B8EA0`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            local action = ACTIONS[me.action]
            if not action then
                error(string.format("BlastMan action %#x reads past his table (off_80B8F1C)", me.action))
            end
            action(me)
        else
            me:free()
        end
        me:update_sprite_while_dimmed()
    end,
}

-- `sub_80B9014`: BlastMan for `user`, reporting to `controller` (the
-- BlastMan chips' navi hook, `off_802CD5C[12]`). (His position is the
-- spawner's registers, all of which his init overwrites.)
function blastman.summon(user: Object, controller: Object, spec: NaviChipSpec): Object?
    local o = navi_chips.spawn(blastman.kind, user, spec)
    if not o then
        return nil
    end
    local s = o.state :: State
    s.controller = controller
    return o
end

return blastman
