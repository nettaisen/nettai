--!strict
-- DblBeast's Falzar beast (the original's actor object #0x34,
-- `sub_80C2138`; docs/engine/dimming-chip-effects.md §19): it appears
-- behind the user wearing its head (its identity's parts), lists the
-- enemy's bodies (./beast) and attacks from next to them: a breath (eight
-- hits two panels ahead) and two wing strikes; then it takes its head off,
-- goes, and tells the controller.

local beast = require("./beast")
local attachment = require("../../objects/attachment")
local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")
local falzar_beast = require("../../navis/megaman/forms/falzar_beast")["falzar-beast"]
local parts = require("../../lib/swords/parts")

-- The slash over two panels.
local LONG_SLASH = new.effect { sprite = asset.sprite("sword-slash"), anim = 1, palette = 0 }
-- The slash over a column.
local WIDE_SLASH = new.effect { sprite = asset.sprite("sword-slash"), anim = 0, palette = 0 }

local falzar = {}

local SPRITE = asset.sprite("falzar-beast")
-- Its identity: Falzar Beast Out's (its parts and the attach points its
-- attachments take).
local IDENTITY = assert(falzar_beast.identity)

-- The attacks' patterns (`byte_80C25B8`, `byte_80C25C0`, `byte_80C25D0`).
local BREATH: DblBeastPattern = { { 0, -2, 0 }, { 1, 2, 0 } }
local WING: DblBeastPattern = { { 0, -1, 0 }, { 1, 1, 0 }, { 0, -2, 0 }, { 1, 2, 0 } }
local STRIKE: DblBeastPattern = { { 0, -1, 0 }, { 1, 1, 0 }, { 0, -1, -1 }, { 1, 1, -1 }, { 0, -1, 1 }, { 1, 1, 1 } }

-- The hits (`byte_80C2640`, `byte_80C2654`, `dword_80C2664`).
local HITS: { [string]: DblBeastHit } = {
    breath = { ahead = 2, region = regions.single, spark = sparks.heavy, is = collision.attack, damage = 10 },
    wing = { ahead = 1, region = regions.long, spark = sparks.heavy, is = collision.attack, damage = 50 },
    strike = { ahead = 1, region = regions.wide, spark = sparks.heavy, is = collision.attack, damage = 50 },
}

-- The breath's maw (attachment row 0x21, the fan Tornado holds) and its
-- gust two panels ahead (effect 0x59, 30 ticks).
local MAW = attachment.look { sprite = asset.sprite("hand-fan"), palette = 0, attach_point = 9 }
local GUST = new.effect { sprite = asset.sprite("tornado"), anim = 0, palette = 0 }
local GUST_TICKS = 0x1E

local SOUND_BREATH = asset.sound("aqua-surge")
local SOUND_WING = asset.sound("sword-swing")

local ANIM_IDLE, ANIM_WING, ANIM_BREATH, ANIM_LEAVE = 0, 5, 0xA, 4
local BREATH_EVERY, WING_AT = 4, 0x12
local LIST_TICKS, LEAVE_TICKS = 0x3C, 3

-- `sub_80C26A0`: an effect on the panel in front, 16 pixels up, facing
-- the beast's way.
local function effect_ahead(me: Object, fx: Effect)
    local cx, cy = field.center(me.panel_x + me.facing, me.panel_y)
    battle.effect(Vec3.new(cx, cy, 0x10_0000), fx, bit32.bxor(me.alliance, me.flip))
end

-- The attack ends: no miss, the next action.
local function finish(me: Object, next: number)
    (me.state :: DblBeastState).missed = false
    me.action, me.phase, me.phase_init = next, 0, 0
end

-- `sub_80B8E58`: an attachment ends.
local function detach(a: Object?)
    if a then
        a:set_lifecycle("destroy")
    end
end

-- A missed attack waits out its ticks.
local function missed_wait(me: Object, next: number): boolean
    if not (me.state :: DblBeastState).missed then
        return false
    end
    local t = me.timer - 1
    me.timer = t
    if t <= 0 then
        finish(me, next)
    end
    return true
end

-- `sub_80C2216`: the targets; 61 ticks; then the breath, or the end when
-- there is no target.
local function list(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 4
        me.anim = ANIM_IDLE
        beast.list_targets(me)
        me.timer = LIST_TICKS
        return
    end
    local t = me.timer - 1
    me.timer = t
    if t > 0 then
        return
    end
    finish(me, if (me.state :: DblBeastState).count ~= 0 then 8 else 0x1C)
end

-- `sub_80C2256`: the breath, a hit every 4 ticks (a miss doesn't go back
-- to rest).
local function breath(me: Object)
    local s = me.state :: DblBeastState
    if me.phase_init == 0 then
        me.phase_init = 4
        me.timer = beast.TICKS
        if beast.take_panel(me, BREATH, false) then
            me.anim = ANIM_BREATH
            attachment.attach(me, MAW, "held", { anim = 1, while_dimmed = true })
            -- sub_80C26C8.
            local cx, cy = field.center(me.panel_x + 2 * me.facing, me.panel_y)
            local gust = battle.effect(Vec3.new(cx, cy, 0), GUST)
            if gust then
                gust.timer = GUST_TICKS
            end
            beast.hit(me, HITS.breath)
            battle.play_sound(SOUND_BREATH)
            me.timer2 = BREATH_EVERY
        end
        return
    end
    if missed_wait(me, 0xC) then
        return
    end
    local t = me.timer - 1
    me.timer = t
    if t > 0 then
        local t2 = me.timer2 - 1
        me.timer2 = t2
        if t2 <= 0 then
            me.timer2 = BREATH_EVERY
            beast.hit(me, HITS.breath)
        end
        return
    end
    me.anim = ANIM_IDLE
    detach(s.held)
    finish(me, 0xC)
end

-- `sub_80C2324` and `sub_80C23F0`: a wing strike from the pattern's panel
-- (back to rest on a miss); its swipe and hit 18 ticks in.
local function strike(me: Object, pattern: DblBeastPattern, swipe: Effect, hit: DblBeastHit, next: number)
    local s = me.state :: DblBeastState
    if me.phase_init == 0 then
        me.phase_init = 4
        me.timer = beast.TICKS
        if beast.take_panel(me, pattern, true) then
            battle.play_sound(SOUND_WING)
            me.anim = ANIM_WING
            attachment.attach(me, parts.blade, "held", { anim = 0xD, while_dimmed = true })
        else
            beast.rest(me, false)
        end
        return
    end
    if missed_wait(me, next) then
        return
    end
    local t = me.timer - 1
    me.timer = t
    if t == WING_AT then
        effect_ahead(me, swipe)
        beast.hit(me, hit)
        return
    end
    if t > 0 then
        return
    end
    me.anim = ANIM_IDLE
    detach(s.held)
    finish(me, next)
end

-- `sub_80C2498`: the last panel let go, 3 ticks, then its head off and
-- gone.
local function leave(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 4
        me:unreserve_panel(me.future_panel_x, me.future_panel_y)
        me.anim = ANIM_LEAVE
        me.timer = LEAVE_TICKS
        return
    end
    local t = me.timer - 1
    me.timer = t
    if t <= 0 then
        me:remove_parts_of(me)
        beast.leave(me, "falzar_out")
    end
end

-- `sub_80C21AC`: by action (`off_80C21C0`).
local function tick(me: Object)
    local a = me.action
    if a == 0 then
        beast.appear(me)
    elseif a == 4 then
        list(me)
    elseif a == 8 then
        breath(me)
    elseif a == 0xC then
        beast.wait(me, 0x10)
    elseif a == 0x10 then
        strike(me, WING, LONG_SLASH, HITS.wing, 0x14)
    elseif a == 0x14 then
        beast.wait(me, 0x18)
    elseif a == 0x18 then
        strike(me, STRIKE, WIDE_SLASH, HITS.strike, 0x1C)
    elseif a == 0x1C then
        leave(me)
    else
        error(string.format("DblBeast's Falzar: action %#x reads past off_80C21C0", a))
    end
end

falzar.kind = new.kind {
    id = "dblbeast/falzar",
    pool = "actor",
    state = beast.STATE,
    -- `sub_80C2138`: by state, then the sprite (stepping while dimmed).
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- sub_80C215C: at rest behind the user, its head on (its
            -- identity's parts, stepping even while paused); its update
            -- first runs next tick.
            beast.init(me, SPRITE, false, IDENTITY)
            me:add_parts_of(me, false, true)
            me:set_lifecycle("update")
        elseif l == "update" then
            tick(me)
        else
            me:free()
            return
        end
        me:update_sprite_while_dimmed()
    end,
}

-- `sub_80C24D2`: Falzar's beast for `user`.
function falzar.spawn(controller: Object, user: Object, element: number, base: number): Object?
    return beast.spawn(falzar.kind, controller, user, element, base, "falzar_out")
end

return falzar
