--!strict
-- Colonel, the navi of the Colonel chips and of CrossDiv (the original's
-- actor object #0x01, `sub_80B8210`), brought by their navi chip
-- controller. He fades in on his user's panel with his cape (a form
-- overlay of his own sprite) and leaves at once if the panel isn't solid;
-- he draws his sword (in his first related slot, which held his user) and
-- strikes his chip's strike: the Colonel chips' ScreenDivide, an X of hit
-- regions over the middle row of the first column ahead that is wholly
-- the enemy's (`colonel.screen_divide`); CrossDiv's own (chips/crossdiv).
-- A camera shake. Then he goes, and the controller moves on. See
-- docs/engine/chips.md §3.6.22.

local attachment = require("../../objects/attachment")
local collision = require("../../rules/collision")

-- What a chip's strike does as it lands (`colonel.strike`, its own: the
-- original picks it by the chip's third parameter).
export type Strike = { run: (me: Object) -> () }

-- What a chip summons him with: his palette (the chips' first parameter
-- picks it), and his strike.
export type Summon = { palette: number, strike: Strike }

export type State = {
    -- His palette, and his strike (his chip's).
    palette: number,
    strike: Strike,
    -- The gray of his color shader, fading in (object +0x19).
    gray: number,
    -- His cape (ExtraVars[0]).
    cape: Object?,
    -- The controller waiting for him (its pointer sits in ExtraVars[1]).
    controller: Object?,
}

local colonel = {}

local SPRITE = asset.sprite("colonel")
local SOUND_APPEAR = asset.sound("appear")
local SOUND_SPIN = asset.sound("spin")

-- His cape (`dword_80B85C4`): his own sprite, stepping while dimmed, his
-- animation plus 0x0D, his palette.
local CAPE: FormOverlaySpec =
    { sprite = SPRITE, stepping = "while_dimmed", anim_offset = 0x0D, nudged = false, owner_palette = true }

-- His sword (`dword_80B85C8`: the original's attachment 0x1F, animation
-- 0x1C, animating while dimmed).
local SWORD = attachment.look { sprite = asset.sprite("colonel-sword"), palette = 0 }
local SWORD_ANIM = 0x1C

-- ScreenDivide's X (region 0x29: the diagonals of a column of three
-- panels, three columns deep), and its effect (0x38).
local SCREEN_DIVIDE = new.region {
    panels = { { 0, -1 }, { 1, -1 }, { 2, -1 }, { 1, 0 }, { 0, 1 }, { 1, 1 }, { 2, 1 } },
}
local DIVIDE = new.effect { sprite = asset.sprite("colonel-effect"), anim = 3, palette = 0 }

-- His fade-in's starting gray.
local FADE_GRAY = 0x1F

-- The controller calls his spawner through r3, which his position takes
-- (his init sets it all).
local SPAWNER_ADDRESS = 0x080B84ED

-- A panel he stands on must be solid (as a word).
local SOLID = 0x10

-- The column ScreenDivide looks for (`byte_80B8494`): all three of its
-- panels the enemy's (panel flag 0x20 set for side 0, clear for side 1).
local ENEMY_COLUMN = { [0] = { 0x20, 0 }, { 0, 0x20 } }

-- When his strike lands (the timer before its tick's count down).
local STRIKE_TICK = 0x20

-- `sub_80B8230`: on his user's panel, faded out (gray 0x7FFF shader),
-- with his cape.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    sprite.palette = (me.state :: State).palette
    sprite.color_shader = 0x7FFF;
    (me.state :: State).cape = battle.form_overlay(me, CAPE)
    me:set_lifecycle("update")
end

-- Action 0 (`sub_80B82C4`): appear.
-- `sub_80B82E0`: shown; he fades in from gray 31, 2 a tick, flickering.
local function fade_in(me: Object)
    local s = me.state :: State
    me.visible = true
    if me.phase_init == 0 then
        s.gray = FADE_GRAY
        battle.play_sound(SOUND_APPEAR)
        me.timer = 0
        me.phase_init = 4
        return
    end
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

-- `sub_80B8338`: 30 ticks; on to the strike, or away if his panel isn't
-- solid.
local function ready(me: Object)
    if me.phase_init == 0 then
        me.timer = 0x1E
        me.phase_init = 4
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    me:set_action(if field.check(me.panel_x, me.panel_y, SOLID, 0) then 4 else 8)
end

-- `sub_80B845C`: ScreenDivide's column: the first ahead wholly the
-- enemy's. (The game searches on past the field forever if there is
-- none.)
local function enemy_column(me: Object): number
    local want = ENEMY_COLUMN[me.alliance]
    local x = me.panel_x + me.facing
    while true do
        local n = 0
        for y = 1, 3 do
            if field.check(x, y, want[1], want[2]) then
                n += 1
            end
        end
        if n >= 3 then
            return x
        end
        x += me.facing
        if x < -1 or x > 8 then
            error("Colonel's column search (sub_80B845C) runs off the field")
        end
    end
end

-- A hit region on (x, y) 16 pixels up, running while dimmed
-- (`sub_80C53A6`): `region`, no hit spark.
function colonel.hit(me: Object, x: number, y: number, region: Region)
    local box = battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = 0,
        z = 0x10_0000,
        region = region,
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

-- The column of the enemy's back row, where he doesn't strike.
function colonel.at_back(me: Object): boolean
    return bit32.bxor(me.alliance, 1) * 5 + 1 == me.panel_x
end

-- A chip's strike, as it lands.
function colonel.strike(run: (me: Object) -> ()): Strike
    return new.record("colonel-strike", { run = run })
end

-- `sub_80B8512`: ScreenDivide, the Colonel chips' strike: an X over the
-- middle of the enemy's first column, its effect a column on, a 20-tick
-- flash.
colonel.screen_divide = colonel.strike(function(me: Object)
    if colonel.at_back(me) then
        return
    end
    local x = enemy_column(me)
    local y = 2
    colonel.hit(me, x, y, SCREEN_DIVIDE)
    local cx, cy = field.center(x + me.facing, y)
    battle.effect(Vec3.new(cx, cy, 0), DIVIDE, me.alliance)
    battle.palette_flash(0, 0x14, true, false)
end)

-- Action 4 (`sub_80B8376`): the strike.
-- `sub_80B8390`: 40 ticks raising his hand (animation 6), then his sword
-- out (animation 5).
local function draw(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 1
        me.timer = 0x28
    end
    me.anim = 6
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    me.anim = 5
    me.phase, me.phase_init = 4, 0
    attachment.attach(me, SWORD, "related", { anim = SWORD_ANIM, while_dimmed = true })
end

-- `sub_80B83C0`: his swing's sound; at 32 left the strike and a camera shake
-- (magnitude 3, 35 ticks); 40 ticks.
local function strike(me: Object)
    if me.phase_init == 0 then
        battle.play_sound(SOUND_SPIN)
        me.timer = 0x28
        me.phase_init = 4
    end
    if me.timer == STRIKE_TICK then
        (me.state :: State).strike.run(me)
        battle.shake_camera(3, 0x23)
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_action(8)
    end
end

-- Action 8 (`sub_80B8406`): he goes (his sword with his first related
-- slot); 5 ticks, then hidden, his cape off.
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
    local cape = (me.state :: State).cape
    if cape then
        cape:set_lifecycle("destroy")
    end
    me:set_lifecycle("destroy")
end

local PHASES: { [number]: { [number]: (Object) -> () } } = {
    [0] = { [0] = fade_in, [4] = ready },
    [4] = { [0] = draw, [4] = strike },
}

-- `sub_80B829C`: the action (and phase); his sprite, while dimmed too; his
-- color shader from the gray (`sub_80B844A`).
local function update_state(me: Object)
    if me.action == 8 then
        leave(me)
    else
        local phases = PHASES[me.action]
        if not phases then
            error(string.format("Colonel action %#x reads past his table (off_80B82B4)", me.action))
        end
        local f = phases[me.phase]
        if not f then
            error(string.format("Colonel action %#x phase %#x reads past its table", me.action, me.phase))
        end
        f(me)
    end
    me:update_sprite_while_dimmed()
    me.sprite.color_shader = (me.state :: State).gray * 0x421
end

-- `sub_80B843C`: the controller moves on, and he is freed.
local function finish(me: Object)
    local controller = (me.state :: State).controller
    if controller then
        navi_chip.navi_left(controller)
    end
    me:free()
end

colonel.kind = new.kind {
    id = "colonel/navi",
    pool = "actor",
    state = {
        gray = "u8",
        cape = "object",
        controller = "object",
        palette = "u8",
        strike = "record:colonel-strike",
    },
    -- `sub_80B8210`.
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

-- `sub_80B84EC`: the Colonel chips' and CrossDiv's navi hook
-- (`off_802CD5C[17]`): Colonel for `user`, reporting to `controller`.
function colonel.summon(args: Summon): (user: Object, controller: Object, spec: NaviChipSpec) -> Object?
    return function(user: Object, controller: Object, spec: NaviChipSpec): Object?
        local o = battle.spawn(colonel.kind, Vec3.new(spec.panel_y, spec.element, SPAWNER_ADDRESS))
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
        s.palette = args.palette
        s.strike = args.strike
        return o
    end
end

return colonel
