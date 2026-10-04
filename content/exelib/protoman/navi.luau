--!strict
-- ProtoMan, the navi of the ProtoMan chips, as BN5 and BN6 share him
-- (BN6's actor object #0x3C, `sub_80C26F0`; BN5's #0x3C), brought by their
-- navi chip controller. He fades
-- in on his user's panel (and leaves at once if it isn't solid); then,
-- column by column ahead and row by row, he finds each enemy navi with a
-- free panel (or his user's) in front of it, blinks there and slashes it
-- with his sword (a column of three and a slash drawn over it). When none
-- is left he goes, and the controller moves on. His sword sits in his
-- first related slot, which held his user until his first slash (so later
-- searches find no user there). See docs/engine/chips.md §3.6.21.
--
-- Each game makes his kind (`protoman.make`, a `ProtoManKind`: his sounds,
-- attachment kind and slash's collision types); his look (sprite, palette,
-- sword, the slash's effect, his spawner's address) is a chip's
-- (`summon_with(look)`).

local attachment = require("../attachment/attachment")
local regions = require("../regions")

export type State = {
    -- The gray of his color shader, fading in (object +0x19).
    gray: number,
    -- The controller waiting for him (its pointer sits in ExtraVars[1]).
    controller: Object?,
    -- Whether the next search starts at the top of its column
    -- (ExtraVars[2]), and his facing when he came (ExtraVars[3]).
    fresh: number,
    flip: number,
    -- His game's look (`protoman.look`).
    look: ProtoManLook,
}

local protoman = {}

-- A game's ProtoMan's look.
function protoman.look(look: ProtoManLook): ProtoManLook
    return define.record("protoman-look", look)
end

-- A game's ProtoMan: his kind, and the chips' navi hook of a look.
function protoman.make(spec: ProtoManKind): {
    kind: Kind,
    summon_with: (look: ProtoManLook) -> (user: Object, controller: Object, spec: NaviChipSpec) -> Object?,
}
    -- His fade-in's starting gray.
    local FADE_GRAY = 0x1F

    -- What he looks for (`off_80C2904`: an enemy navi's body), what a panel
    -- he lands on must be (solid) and be free of (as words).
    local ENEMY_NAVI = { [0] = 0x0400_0000, 0x0800_0000 }
    local SOLID = 0x10
    local OCCUPIED = 0x0F80_0000

    -- When his slash lands (the timer before its tick's count down).
    local SLASH_TICK = 0xA

    -- Enter the phase with a timer (PhaseInitialized), or count down; true
    -- once the timer is 0 or below.
    local function countdown(me: Object, ticks: number): boolean
        if me.phase_init == 0 then
            me.timer = ticks
            me.phase_init = 4
            return false
        end
        local left = me.timer - 1
        me.timer = left
        return left <= 0
    end

    -- Action 0 (`sub_80C2798`): appear.
    -- `sub_80C27B4`: he fades in from gray 31, 2 a tick, flickering.
    local function fade_in(me: Object)
        local s = me.state :: State
        if me.phase_init == 0 then
            s.gray = FADE_GRAY
            battle.play_sound(spec.appear_sound)
            me.timer = 0
            me.phase_init = 4
            return
        end
        me.visible = true
        local t = me.timer + 1
        me.timer = t
        if bit32.band(t, 1) ~= 0 then
            me.visible = false
        end
        local gray = s.gray - 2
        s.gray = gray
        if gray > 0 then
            return
        end
        me.visible = true
        s.gray = 0
        me.phase, me.phase_init = 4, 0
    end

    -- `sub_80C2804`: 30 ticks; gone if his panel isn't solid; else the search
    -- starts at the top of the column ahead.
    local function ready(me: Object)
        if not countdown(me, 0x1E) then
            return
        end
        if not field.check(me.panel_x, me.panel_y, SOLID, 0) then
            me:set_lifecycle("destroy")
            return
        end
        me.future_panel_x = me.panel_x + me.facing
        me.future_panel_y = 1
        local s = me.state :: State
        s.fresh = 1
        s.flip = bit32.bxor(me.alliance, me.flip)
        me:set_action(4)
    end

    -- The panel in front of an enemy navi at (x, y) will hold him: his
    -- user's (while his first related slot still holds it) or a free solid
    -- one.
    local function can_land(me: Object, x: number, y: number): boolean
        local lx = x - me.facing
        local user = me.related1
        if user and lx == user.panel_x and y == user.panel_y then
            return true
        end
        return field.check(lx, y, SOLID, OCCUPIED)
    end

    -- Action 4 (`sub_80C2854`): the slashes.
    -- `sub_80C2878`: the next enemy navi, from the last one's next row (or
    -- the top of the column), column by column ahead; none left: action 8.
    local function search(me: Object)
        local s = me.state :: State
        local x, y = me.future_panel_x, me.future_panel_y
        if field.valid(x, y) then
            local enemy = ENEMY_NAVI[me.alliance]
            if s.fresh ~= 0 then
                y = 1
            else
                y += 1
            end
            while true do
                while y <= 3 do
                    if field.check(x, y, enemy, 0) and can_land(me, x, y) then
                        me.future_panel_x, me.future_panel_y = x, y
                        s.fresh = 0
                        me.phase, me.phase_init = 4, 0
                        return
                    end
                    y += 1
                end
                x += me.facing
                if x < 1 or x > 6 then
                    break
                end
                y = 1
            end
        end
        me:set_action(8)
    end

    -- `sub_80C290C`: he blinks out (animation 4; his first related slot
    -- cleared: his sword goes); 3 ticks.
    local function blink_out(me: Object)
        if me.phase_init == 0 then
            me.anim = 4
            me.related1 = nil
        end
        if countdown(me, 3) then
            me.phase, me.phase_init = 8, 0
        end
    end

    -- `sub_80C2934`: in front of the target; 3 ticks.
    local function blink_in(me: Object)
        if me.phase_init == 0 then
            me.anim = 4
            me.timer = 3
            me.panel_x = me.future_panel_x - me.facing
            me.panel_y = me.future_panel_y
            me:set_coordinates_from_panel()
            me.phase_init = 4
            return
        end
        local left = me.timer - 1
        me.timer = left
        if left <= 0 then
            me.phase, me.phase_init = 0xC, 0
        end
    end

    -- `sub_80C296C`: the slash (his sword in his first related slot): at 10
    -- left, a hit on a column of three 16 pixels up on the panel ahead
    -- (running while dimmed, `sub_80C53A6`) and its effect; 20 ticks, then
    -- the next search.
    local function slash(me: Object)
        if me.phase_init == 0 then
            me.anim = 5
            me.anim_loaded = 6
            local look = (me.state :: State).look
            attachment.attach(spec.attachment, me, look.sword, "related", { anim = look.sword_anim, while_dimmed = true })
            battle.play_sound(spec.swing_sound)
            me.timer = 0x14
            me.phase_init = 4
            return
        end
        if me.timer == SLASH_TICK then
            local x, y = me.panel_x + me.facing, me.panel_y
            local box = battle.hitbox(me, {
                panel_x = x,
                panel_y = y,
                element = 0,
                z = 0x10_0000,
                region = regions.wide,
                hit_effect = nil,
                target = spec.target,
                self_type = spec.self_type,
                damage = me.damage,
                stamina = me.stamina,
                hit_mod = 3,
            })
            if box then
                box.run_while_dimmed = true
            end
            local cx, cy = field.center(x, y)
            local s = me.state :: State
            battle.effect(Vec3.new(cx, cy, 0x10_0000), s.look.slash, s.flip)
        end
        local left = me.timer - 1
        me.timer = left
        if left <= 0 then
            me.phase, me.phase_init = 0, 0
        end
    end

    -- Action 8 (`sub_80C29FC`): he goes (his sword with his first related
    -- slot); 5 ticks, then hidden, and freed.
    local function leave(me: Object)
        if me.phase == 0 then
            me.related1 = nil
            me.anim, me.anim_loaded = 4, 0
            me.timer = 5
            me.phase = 4
            return
        end
        local left = me.timer - 1
        me.timer = left
        if left > 0 then
            return
        end
        me.visible = false
        me:set_lifecycle("destroy")
    end

    local ACTIONS: { [number]: { [number]: (Object) -> () } } = {
        [0] = { [0] = fade_in, [4] = ready },
        [4] = { [0] = search, [4] = blink_out, [8] = blink_in, [0xC] = slash },
    }

    -- `sub_80C2770`: the action (and phase); his sprite, while dimmed too;
    -- his color shader from the gray (`sub_80C2A3A`).
    local function update_state(me: Object)
        if me.action == 8 then
            leave(me)
        else
            local phases = ACTIONS[me.action]
            if not phases then
                error(string.format("ProtoMan action %#x reads past his table (off_80C2788)", me.action))
            end
            local f = phases[me.phase]
            if not f then
                error(string.format("ProtoMan action %#x phase %#x reads past its table", me.action, me.phase))
            end
            f(me)
        end
        me:update_sprite_while_dimmed()
        me.sprite.color_shader = (me.state :: State).gray * 0x421
    end

    -- `sub_80C2710`: on his user's panel, faded out (gray); then his first
    -- update at once.
    local function init(me: Object)
        local look = (me.state :: State).look
        local sprite = me.sprite
        sprite:load(look.sprite)
        sprite.shadow = "ground"
        me.anim = 0
        me.anim_loaded = 0
        sprite:set_animation(0)
        me:set_coordinates_from_panel()
        local p = me.pos
        me.pos = Vec3.new(p.x, p.y, 0)
        sprite:set_flip(bit32.bxor(me.alliance, me.flip))
        sprite.palette = look.palette
        sprite.color_shader = 0x7FFF
        me.visible = true
        me:set_lifecycle("update")
        update_state(me)
    end

    -- `sub_80C2A2C`: the controller moves on, and he is freed.
    local function finish(me: Object)
        local controller = (me.state :: State).controller
        if controller then
            navi_chip.navi_left(controller)
        end
        me:free()
    end

    local kind = define.kind {
        id = spec.id,
        pool = "actor",
        state = { gray = "u8", controller = "object", fresh = "u8", flip = "u8", look = "record:protoman-look" },
        -- `sub_80C26F0`.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me)
            elseif l == "update" then
                update_state(me)
            else
                finish(me)
            end
        end,
    }

    -- `sub_80C2A4C`: ProtoMan of `look` for `user`, reporting to `controller`
    -- (the ProtoMan chips' navi hook, `off_802CD5C[1]`).
    local function summon_with(look: ProtoManLook): (user: Object, controller: Object, spec: NaviChipSpec) -> Object?
        return function(user: Object, controller: Object, spec: NaviChipSpec): Object?
            local o = battle.spawn(kind, Vec3.new(spec.panel_y, spec.element, look.spawner_address))
            if not o then
                return nil
            end
            o.panel_x, o.panel_y = spec.panel_x, spec.panel_y
            o.element = spec.element
            o.alliance, o.flip = user.alliance, user.flip
            o.related1 = user
            o:set_damage_word(spec.damage)
            local s = o.state :: State
            s.controller = controller
            s.look = look
            return o
        end
    end

    return { kind = kind, summon_with = summon_with }
    end

return protoman
