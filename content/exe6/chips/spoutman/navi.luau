--!strict
-- SpoutMan, the navi of the SpoutMan chips (the original's actor object
-- #0x09, `sub_80B94BC`), brought by their navi chip controller. He appears in his
-- water (his layer: the parts of his actor record, AI index 6), and
-- attacks by where he stands:
--
--   away from his back columns: he throws a water ball two columns ahead
--   (./ball), which splashes there and one column beyond (./splash);
--   in them: the water under him spouts (./pillar) and a geyser erupts in
--   the first column ahead holding an enemy navi, or the field's edge,
--   hitting that column and the trail to it three times (./geyser, its
--   marks ./mark).
--
-- Then he leaves, and the controller moves on. See docs/engine/chips.md
-- §3.6.11.

local spoutman_navi = require("../../navis/spoutman").spoutman
local panels = require("@exelib/panels")
local ball = require("./ball")
local geyser = require("./geyser")
local pillar = require("./pillar")
local navi_chips = require("../../lib/navi_chips/navi")

export type State = {
    -- The controller waiting for him (its pointer sits in his collision
    -- data slot).
    controller: Object?,
    -- The water spouting under him (ExtraVars[0], which it clears when it
    -- goes).
    pillar: Object?,
}

local spoutman = {}

local SPRITE = asset.sprite("spoutman")
local SOUND_APPEAR = asset.sound("appear")
local SOUND_BUBBLE = asset.sound("bubble")
local SOUND_SPOUT_MAN = asset.sound("spout-man")

-- What he wears (his layer, `sub_8010F7A`): the link navi's own parts.
local IDENTITY = spoutman_navi.identity

-- What his geyser looks for, by side (`off_80B97B8`: an enemy navi's
-- body).
local ENEMY_NAVI = { [0] = 0x0400_0000, 0x0800_0000 }

-- How long his geyser lasts.
local GEYSER_TICKS = 0x5A
-- His water ball: the navi chip's (it acts while dimmed), cracking the
-- panel it lands on.
local BALL: WaterBall = { while_dimmed = true, cracks = true }
-- His throw's splash where he stands (effect 0x2A).
local THROW = new.effect { sprite = asset.sprite("spout-man-effect"), anim = 0, palette = 0 }

-- The controller calls his spawner through r3, which his position takes:
-- its address with the Thumb bit, whose fraction his Z keeps.
local SPAWNER_ADDRESS = 0x080B9751

-- The pillar's signals (its phase_init byte).
local SPOUT, DONE = 1, 2

-- `sub_80B94E0`: on his panel, on the ground (Z's whole part cleared).
local function init(me: Object)
    navi_chips.stand(me, SPRITE, "ground")
    me:set_lifecycle("update")
end

-- Enter the phase (phase 4, a word store of the timers), or count down;
-- true once the timer is 0 or below.
local function countdown(me: Object, ticks: number): boolean
    if me.phase == 0 then
        me.phase = 4
        me.timer, me.timer2 = ticks, 0
        return false
    end
    local left = me.timer - 1
    me.timer = left
    return left <= 0
end

local function damage_word(me: Object): number
    return bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
end

-- `sub_80B954C`: he appears with his layer; 6 ticks; on to standing, or
-- straight to leaving if his panel can't hold him.
local function appear(me: Object)
    if me.phase == 0 then
        me.anim = 3
        battle.play_sound(SOUND_APPEAR)
        me:add_parts(IDENTITY, 1)
        me.visible = true
    end
    if not countdown(me, 6) then
        return
    end
    me:set_action(if navi_chips.has_footing(me) then 4 else 0x18)
end

-- `sub_80B95AC`: 30 ticks standing; then, away from his back columns, the
-- water ball; in them, the geyser.
local function stand(me: Object)
    if me.phase == 0 then
        me.anim = 0
    end
    if not countdown(me, 0x1E) then
        return
    end
    local x = me.panel_x
    local forward = if me.alliance == 0 then x > 2 else x < 5
    me:set_action(if forward then 8 else 0x10)
end

-- `sub_80B95F2`: 20 ticks winding up the throw.
local function wind_up(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 0x13
        me.timer = 0x14
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_action(0xC)
    end
end

-- `sub_80B961A`: the throw: a water ball and a splash effect where he is;
-- 60 ticks.
local function throw(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 0x14
        ball.spawn(me, me.panel_x, me.panel_y, me.element, BALL, damage_word(me))
        local p = me.pos
        battle.effect(Vec3.new(p.x, p.y, 0), THROW, bit32.bxor(me.alliance, me.flip))
        battle.play_sound(SOUND_BUBBLE)
        me.timer = 0x3C
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_action(0x18)
    end
end

-- `sub_80B966C`: the water under him rises (its pillar); at 57 left he
-- raises his arms, at 27 the pillar spouts; then the geyser.
local function rise(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 0xF
        pillar.spawn(me, me.panel_x, me.panel_y, true)
        me.timer = 0x42
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left == 0x39 then
        me.anim = 0x10
        return
    end
    if left == 0x1B then
        local water = (me.state :: State).pillar
        if water then
            water.phase_init = SPOUT
        end
        return
    end
    if left <= 0 then
        me:set_action(0x14)
    end
end

-- `sub_80B9776`: the column his geyser aims at: the first one ahead with
-- an enemy navi in it, or the field's edge.
local function target_column(me: Object): number
    local i = 1
    while true do
        local x = me.panel_x + me.facing * i
        if #panels.in_column(x, ENEMY_NAVI[me.alliance], 0) ~= 0 or x <= 1 or x >= 6 then
            return x
        end
        i += 1
    end
end

-- `sub_80B96C2`: the geyser, 90 ticks; then his pillar is done.
local function erupt(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 0x15
        battle.play_sound(SOUND_SPOUT_MAN)
        me.timer = GEYSER_TICKS
        local target = target_column(me)
        -- r3 (4, the geyser's Param1) becomes its Z.
        local what: Geyser = { while_dimmed = true, ticks = GEYSER_TICKS, target = target }
        geyser.spawn(me, me.panel_x, me.panel_y, me.element, what, damage_word(me), 4)
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    me.anim = 0
    local water = (me.state :: State).pillar
    if water then
        water.phase_init = DONE
    end
    me:set_action(0x18)
end

-- `sub_80B971E`: 9 ticks (to -1), then he goes (his layer with him) and
-- the controller moves on.
local function leave(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 4
        me.timer, me.timer2 = 8, 0
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left >= 0 then
        return
    end
    me:remove_parts(IDENTITY)
    navi_chips.leave(me, (me.state :: State).controller)
end

local ACTIONS = {
    [0] = appear,
    [4] = stand,
    [8] = wind_up,
    [0xC] = throw,
    [0x10] = rise,
    [0x14] = erupt,
    [0x18] = leave,
}

spoutman.kind = new.kind {
    id = "spoutman/navi",
    pool = "actor",
    state = { controller = "object", pillar = "object" },
    -- `sub_80B94BC`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            local action = ACTIONS[me.action]
            if not action then
                error(string.format("SpoutMan action %#x reads past his table (off_80B9530)", me.action))
            end
            action(me)
        else
            me:free()
        end
        me:update_sprite_while_dimmed()
    end,
}

-- `sub_80B9750`: SpoutMan for `user`, reporting to `controller` (the
-- SpoutMan chips' navi hook, `off_802CD5C[7]`). His position is the
-- spawner's registers until his init places him.
function spoutman.summon(user: Object, controller: Object, spec: NaviChipSpec): Object?
    local o = navi_chips.spawn(spoutman.kind, user, spec, SPAWNER_ADDRESS)
    if not o then
        return nil
    end
    local s = o.state :: State
    s.controller = controller
    return o
end

return spoutman
