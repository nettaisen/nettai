--!strict
-- SumnBlk's navi (the original's actor object #0x32, `sub_80C1570`;
-- docs/engine/dimming-chips.md §9.2): it rises from the hole in front of
-- its user; finding a target (the panel in front of one of the other
-- side's actors, `target`), it moves there and claws the column in front
-- of it; then it sinks away. It tells its controller (the holder) when
-- it is gone.

local collision = require("../../rules/collision")
local regions = require("../../lib/regions")

local navi = {}

export type State = {
    -- Its controller, whose `busy` it clears when it is gone (ExtraVars+0
    -- points at the flag).
    holder: Object?,
    -- Its look (+0x64: the chip's variant; palette three a variant).
    variant: number,
    -- The panel it strikes from (+0x65, +0x66).
    target_x: number,
    target_y: number,
}

local SPRITE = asset.sprite("summon-black")
local APPEAR_SOUND, RISE_SOUND, CLAW_SOUND = asset.sound("appear"), asset.sound("bug"), asset.sound("tomahawk-man")
-- The puffs where it sinks and where it comes up (`byte_80C1854`: effect
-- 0x14 by 1, 0x15 by 0), and its claw (effect 0x16).
local PUFF = define.effect { sprite = asset.sprite("puff"), anim = 2, palette = 0 }
local ARRIVE = define.effect { sprite = asset.sprite("puff"), anim = 3, palette = 0 }
local CLAW = define.effect { sprite = asset.sprite("sword-slash"), anim = 0, palette = 0 }
-- The claw drawn upside down (flip bit 1) and in palette +6.
local CLAW_FLIP, CLAW_PALETTE = 2, 6
-- An actor whose look is this stands two panels wide.
local WIDE_LOOK = asset.sprite("wide-navi")
-- Bodies (of either side) on a panel.
local BODIES = 0x0F80_0000

-- The actions (the object's action byte).
local RISE, STRIKE, LEAVE = 0, 4, 8

-- `sub_80C1820`: a puff (`arrive`: the other one) on panel (x, y),
-- turned its way.
local function puff(me: Object, x: number, y: number, arrive: boolean)
    local cx, cy = field.center(x, y)
    battle.effect(Vec3.new(cx, cy, 0), if arrive then ARRIVE else PUFF, bit32.bxor(me.alliance, me.flip))
end

-- The panel in front of `actor` (two ahead for a wide one).
local function in_front(actor: Object): (number, number)
    local step = if actor:name_look_is(WIDE_LOOK) then 2 else 1
    return actor.panel_x + actor.facing * step, actor.panel_y
end

-- `sub_80C18B0`: of the other side's actors, those with no body on the
-- panel in front of them; of those, the ones at least two columns ahead
-- of the user (the controller's column), the nearest kept; of those, the
-- one in the top row (the last on a tie). The panel in front of it, or
-- nil.
local function target(me: Object): (number?, number?)
    local actors = battle.alive_actors(bit32.bxor(me.alliance, 1))
    if #actors == 0 then
        return nil, nil
    end
    local open: { Object } = {}
    for i = #actors, 1, -1 do
        local a = actors[i]
        local x, y = in_front(a)
        if bit32.band(field.flags(x, y), BODIES) == 0 then
            table.insert(open, a)
        end
    end
    if #open == 0 then
        return nil, nil
    end
    local controller = me.related1 :: Object
    local near: { Object } = {}
    local nearest = 0
    for i = #open, 1, -1 do
        local a = open[i]
        local d = (a.panel_x - controller.panel_x) * controller.facing
        if d >= 2 and (#near == 0 or d <= nearest) then
            nearest = d
            table.insert(near, a)
        end
    end
    if #near == 0 then
        return nil, nil
    end
    local chosen = near[1]
    for i = 2, #near do
        if near[i].panel_y <= chosen.panel_y then
            chosen = near[i]
        end
    end
    return in_front(chosen)
end

-- `sub_80C1610`: unseen for 24 ticks, it rises (`sub_80C1630`), waits 30
-- ticks after (`sub_80C166A`), and looks for a target (`sub_80C1698`).
local function rise(me: Object)
    local phase = me.phase
    if phase == 0 then
        if me.phase_init == 0 then
            battle.play_sound(APPEAR_SOUND)
            me.timer = 0x18
            me.phase_init = 4
            return
        end
        me.visible = true
        me.timer = int.u16(me.timer - 1)
        if me.timer == 0 then
            me.anim = 5
            battle.play_sound(RISE_SOUND)
            me.phase, me.phase_init = 4, 0
        end
    elseif phase == 4 then
        if me.phase_init == 0 then
            if bit32.band(me.sprite.frame_flags, 0x80) ~= 0 then
                me.anim = 4
                me.timer = 0x1E
                me.phase_init = 4
            end
            return
        end
        me.timer = int.u16(me.timer - 1)
        if me.timer == 0 then
            me.phase, me.phase_init = 8, 0
        end
    elseif phase == 8 then
        local x, y = target(me)
        -- (A target in column 0 reads as none: the game tests the column.)
        if x and y and x ~= 0 then
            local s = me.state :: State
            s.target_x, s.target_y = int.u8(x), int.u8(y)
            me:set_action(STRIKE)
        else
            me:set_action(LEAVE)
        end
    else
        error(string.format("SumnBlk's navi: rise phase %#x reads past off_80C1624", phase))
    end
end

-- `sub_80C185C`, `sub_80C188C`: its claw on the panel in front, and the
-- hit on that column, which resolves while dimmed.
local function claw(me: Object)
    local x, y = me.panel_x + me.facing, me.panel_y
    local cx, cy = field.center(x, y)
    battle.effect(Vec3.new(cx, cy, 0), CLAW, bit32.bor(CLAW_FLIP, bit32.bxor(me.alliance, me.flip)), CLAW_PALETTE)
    local box = battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = me.element,
        z = 0,
        region = regions.wide,
        -- None.
        hit_effect = nil,
        target = collision.hits_navis,
        self_type = collision.slash,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = 3,
    })
    if box then
        box.run_while_dimmed = true
    end
end

-- `sub_80C16BE`: it sinks and comes up at the target (`sub_80C16E0`),
-- raises its claw (`sub_80C1734`), strikes 10 ticks after the animation
-- (`sub_80C174A`), and leaves 16 ticks after the strike's
-- (`sub_80C1786`).
local function strike(me: Object)
    local phase = me.phase
    if phase == 0 then
        if me.phase_init == 0 then
            me.phase_init = 1
            puff(me, me.panel_x, me.panel_y, false)
            local s = me.state :: State
            me.panel_x, me.panel_y = s.target_x, s.target_y
            me:set_coordinates_from_panel()
            puff(me, me.panel_x, me.panel_y, true)
            me.timer = 2
            me.visible = false
            return
        end
        me.timer = int.u16(me.timer - 1)
        if me.timer == 0 then
            me.visible = true
            me.phase, me.phase_init = 4, 0
        end
    elseif phase == 4 then
        if me.phase_init == 0 then
            me.phase_init = 1
        end
        me.anim = 1
        me.phase, me.phase_init = 8, 0
    elseif phase == 8 or phase == 0xC then
        if me.phase_init == 0 then
            if bit32.band(me.sprite.frame_flags, 0x80) == 0 then
                return
            end
            me.phase_init = 1
            me.timer = if phase == 8 then 0xA else 0x10
        end
        me.timer = int.u16(me.timer - 1)
        if me.timer ~= 0 then
            return
        end
        if phase == 8 then
            me.anim = 2
            claw(me)
            battle.play_sound(CLAW_SOUND)
            me.phase, me.phase_init = 0xC, 0
        else
            me:set_action(LEAVE)
        end
    else
        error(string.format("SumnBlk's navi: strike phase %#x reads past off_80C16D0", phase))
    end
end

-- `sub_80C17B2`: it lets go of its controller, puffs, and is gone 16
-- ticks later, unseen.
local function leave(me: Object)
    if me.phase == 0 then
        me.related1 = nil
        me.timer = 0x10
        puff(me, me.panel_x, me.panel_y, false)
        me.phase = 4
    end
    me.visible = false
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t <= 0 then
        me:set_lifecycle("destroy")
    end
end

-- `sub_80C15EC`: the action, then the sprite (also while dimmed).
local function tick(me: Object)
    local a = me.action
    if a == RISE then
        rise(me)
    elseif a == STRIKE then
        strike(me)
    elseif a == LEAVE then
        leave(me)
    else
        error(string.format("SumnBlk's navi: action %#x reads past off_80C1600", a))
    end
    me:update_sprite_while_dimmed()
end

-- `sub_80C1590`: on its panel in its look, unseen.
local function init(me: Object)
    local s = me.state :: State
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    me:set_coordinates_from_panel()
    me.pos = Vec3.new(me.pos.x, me.pos.y, 0)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    sprite.palette = s.variant * 3
    me.visible = false
    me:set_lifecycle("update")
    tick(me)
end

navi.kind = define.kind {
    id = "sumnblk/navi",
    pool = "actor",
    state = { holder = "object", variant = "u8", target_x = "u8", target_y = "u8" },
    -- `sub_80C1570`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            -- `sub_80C17E6`: its controller is told.
            local holder = (me.state :: State).holder
            if holder then
                holder.state.busy = false
            end
            me:free()
        end
    end,
}

-- `sub_80C17F4`: the navi of `variant` on panel (`x`, `y`) for
-- `controller` (its side and facing), with `element` and the damage word;
-- the controller is busy until it is gone.
function navi.spawn(controller: Object, x: number, y: number, element: number, variant: number, damage: number): Object?
    local o = battle.spawn(navi.kind, Vec3.zero)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.element = element
    o.alliance, o.flip = controller.alliance, controller.flip
    o.related1 = controller
    o:set_damage_word(damage)
    local s = o.state :: State
    s.variant = variant
    s.holder = controller
    controller.state.busy = true
    return o
end

return navi
