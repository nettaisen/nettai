--!strict
-- The TwinLdrs Program Advance's navis, ProtoMan and Colonel: one kind
-- (the original's actor object #0x20, `sub_80BD388`), told apart by
-- `colonel` (its Param1). The navi chip controller brings ProtoMan, who
-- brings Colonel. ProtoMan fades in and slashes each enemy navi he can
-- reach, column by column ahead, blinking in front of it (a wide slash);
-- then he comes back and waits for Colonel. Colonel charges meanwhile, and
-- once ProtoMan is back he strikes an X on the first enemy navi in his
-- row. Then both go, and the controller moves on. See docs/engine/chips.md
-- §3.6.30.

local attachment = require("../../objects/attachment/attachment")
local collision = require("../../rules/collision")
local navi_chips = require("../../lib/navi-chips/navi")
local parts = require("../../lib/swords/parts")
local regions = require("../../lib/regions")

local twinldrs = {}

export type State = {
    -- Colonel (Param1 1), or ProtoMan.
    colonel: boolean,
    -- The controller waiting for ProtoMan (ExtraVars[0]); Colonel has none.
    controller: Object?,
    -- The other one (ExtraVars[1]).
    partner: Object?,
    -- The sword in his hand (ExtraVars[2]).
    held: Object?,
    -- ProtoMan: the next search starts at the top of its column
    -- (ExtraVars[3]).
    fresh: boolean,
    -- Colonel: his charge's glow (ExtraVars[5]).
    glow: Object?,
    -- ProtoMan: the grey of his colour shader, fading in (Param4).
    grey: number,
    -- ProtoMan: how many he has found to slash; Colonel: his charge's step
    -- (Param3).
    count: number,
    -- Where he came in, beside his user: where ProtoMan comes back to.
    home_x: number,
    home_y: number,
    home_z: number,
}

-- How each looks and stands (`byte_80BD464`): his sprite, his actor
-- record's AI index (his parts), and where beside his user he comes in,
-- in pixels (forward, down).
type Look = { sprite: SpriteAsset, ai_index: number, dx: number, dy: number }
local PROTOMAN: Look = { sprite = asset.sprite("protoman"), ai_index = 0xB, dx = 10, dy = 10 }
local COLONEL: Look = { sprite = asset.sprite("colonel"), ai_index = 0x12, dx = -10, dy = -10 }

local SOUND_APPEAR = asset.sound("appear")
local SOUND_STRIKE = asset.sound("spin")
local SOUND_CHARGE, SOUND_CHARGED = asset.sound("buster-charge"), asset.sound("buster-charged")

-- Colonel's sword (attachment 0x1F) and his X's two halves (effects 0x36
-- and 0x37).
local COLONEL_SWORD = attachment.look { sprite = asset.sprite("colonel-sword"), palette = 0 }
local CROSS = {
    define.effect { sprite = asset.sprite("colonel-effect"), anim = 1, palette = 0 },
    define.effect { sprite = asset.sprite("colonel-effect"), anim = 2, palette = 0 },
}

-- Colonel's charge: the glow while it builds (effect 0x4E) and the flash
-- when it is full (0x4F), at his sprite's attach point 0 (`sub_8018810` by
-- his NameID: his sprite's table row, whose point 0 is (0, 0)).
local CHARGING = define.effect { sprite = asset.sprite("charge-glow"), anim = 0, palette = 0 }
local CHARGED = define.effect { sprite = asset.sprite("charge-glow"), anim = 2, palette = 0 }
local GLOW_X, GLOW_Z = 0, 0

-- The afterimages ProtoMan leaves (`sub_80BDA7C`): their colour shader and
-- lifetime.
local AFTERIMAGE_SHADER, AFTERIMAGE_TICKS = 0x8318, 0x1E

-- The controller calls ProtoMan's spawner (`sub_80BD9A2`) through r3,
-- which his position takes (his init sets it all).
local SPAWNER_ADDRESS = 0x080BD9A3

-- What ProtoMan looks for (an enemy navi's body, `off_80BD63C`; Colonel's
-- `off_80BDA68` is the same), and what a panel he lands on must be (solid)
-- and be free of (as words).
local ENEMY_NAVI = { [0] = 0x0400_0000, [1] = 0x0800_0000 }
local SOLID, OCCUPIED = 0x10, 0x0F80_0000

-- Where Colonel strikes when no enemy navi is in his row (`dword_80BDA70`),
-- by side.
local FALLBACK_COLUMN = { [0] = 5, [1] = 2 }

-- ProtoMan's actions; Colonel's (`off_80BD4B0`).
local APPEAR, SLASHES, BACK, LEAVE = 0x0, 0x4, 0x8, 0xC
local STRIKE, COLONEL_LEAVE = 0x4, 0x8

-- How long his phase has left, one tick down (a halfword timer).
local function tick_down(me: Object): number
    local left = me.timer - 1
    me.timer = left
    return left
end

-- The game's flip value for his sprite.
local function flip_of(me: Object): number
    return bit32.bxor(me.alliance, me.flip)
end

-- `sub_80B8E58`: the sword he holds goes (its lifecycle word goes to
-- destroy); the slot keeps it, as the game's ExtraVars[2] does.
local function sword_off(me: Object)
    local sword = (me.state :: State).held
    if sword then
        sword:set_lifecycle("destroy")
    end
end

-- `sub_80BDA7C(anim)`: ProtoMan's afterimages: with `anim` 5 his sword's
-- first, then his own holding `anim`.
local function afterimages(me: Object, anim: number)
    local flip = flip_of(me)
    if anim == 5 then
        battle.afterimage(me, me.pos, {
            sprite = parts.blades.sword.sprite,
            anim = 0xB,
            flip = flip,
            lifetime = AFTERIMAGE_TICKS,
            color_shader = AFTERIMAGE_SHADER,
        })
    end
    battle.afterimage(me, me.pos, {
        sprite = PROTOMAN.sprite,
        anim = anim,
        flip = flip,
        lifetime = AFTERIMAGE_TICKS,
        color_shader = AFTERIMAGE_SHADER,
        shadow = "ground",
    })
end

-- A hit on (x, y) for this navi's damage word (`sub_80C53A6`: it runs
-- while dimmed).
local function hit(me: Object, x: number, y: number, element: number, z: number, region: Region)
    local box = battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = element,
        z = z,
        region = region,
        hit_effect = nil,
        target = collision.hits_navis,
        self_type = collision.attack,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = 3,
    })
    if box then
        box.run_while_dimmed = true
    end
end

-- ---- ProtoMan -----------------------------------------------------------

-- `sub_80BD4DC`: he fades in from grey 31, 2 a tick, flickering.
local function fade_in(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.sprite.color_shader = 0x7FFF
        s.grey = 0x1F
        battle.play_sound(SOUND_APPEAR)
        me.timer = 0
        me.phase_init = 4
        return
    end
    me.visible = true
    local t = me.timer + 1
    me.timer = t
    if bit32.btest(t, 1) then
        me.visible = false
    end
    local grey = s.grey - 2
    if grey > 0 then
        s.grey = grey
        return
    end
    me.visible = true
    s.grey = 0
    me.phase, me.phase_init = 4, 0
end

-- `sub_80BD538`: 30 ticks; gone if his panel isn't solid; else the search
-- starts at the top of the column ahead.
local function ready(me: Object)
    if me.phase_init == 0 then
        me.timer = 0x1E
        me.phase_init = 4
        return
    end
    if tick_down(me) > 0 then
        return
    end
    if not field.solid(me.panel_x, me.panel_y) then
        me:set_action(LEAVE)
        return
    end
    me.future_panel_x = me.panel_x + me.facing
    me.future_panel_y = 1
    local s = me.state :: State
    s.fresh = true
    s.count = 0
    me:set_action(SLASHES)
end

-- `sub_80BD4BC`: his appearing, his colour shader following his grey.
local function appear(me: Object)
    local phase = me.phase
    if phase == 0 then
        fade_in(me)
    elseif phase == 4 then
        ready(me)
    else
        error(string.format("TwinLdrs ProtoMan's appearing phase %#x reads past its table (off_80BD4D4)", phase))
    end
    me.sprite.color_shader = (me.state :: State).grey * 0x421
end

-- The panel in front of an enemy navi at (x, y) will hold him: his
-- user's or a free solid one. (After his first slash his user is gone
-- from his first related slot, and the game reads the user's panel
-- through the null pointer, from the BIOS's open bus, which names no
-- panel on the field.)
local function can_land(me: Object, x: number, y: number): boolean
    local lx = x - me.facing
    local user = me.related1
    if user and lx == user.panel_x and y == user.panel_y then
        return true
    end
    return field.check(lx, y, SOLID, OCCUPIED)
end

-- `sub_80BD5A8`: the next enemy navi, from the last one's next row (or
-- the top of the column), column by column ahead; none left: back.
local function search(me: Object)
    local s = me.state :: State
    local x, y = me.future_panel_x, me.future_panel_y
    if field.valid(x, y) then
        local enemy = ENEMY_NAVI[me.alliance]
        y = if s.fresh then 1 else y + 1
        while true do
            while y <= 3 do
                if field.check(x, y, enemy, 0) and can_land(me, x, y) then
                    me.future_panel_x, me.future_panel_y = x, y
                    s.fresh = false
                    s.count += 1
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
    me:set_action(BACK)
end

-- `sub_80BD644`: he blinks out (his sword goes, and his user from his
-- first related slot); afterimages at the first target's; 3 ticks.
local function blink_out(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.anim = 4
        sword_off(me)
        me.related1 = nil
        me.timer = 3
        if s.count == 1 then
            afterimages(me, 0)
        end
        me.phase_init = 4
        return
    end
    if tick_down(me) <= 0 then
        me.phase, me.phase_init = 8, 0
    end
end

-- `sub_80BD67E`: in front of the target; a tick.
local function blink_in(me: Object)
    if me.phase_init == 0 then
        me.anim = 4
        me.timer = 0
        me.panel_x = me.future_panel_x - me.facing
        me.panel_y = me.future_panel_y
        me:set_coordinates_from_panel()
        me.phase_init = 4
        return
    end
    if tick_down(me) <= 0 then
        me.phase, me.phase_init = 0xC, 0
    end
end

-- `sub_80BD6B6`: the slash (animation 5, restarted; his sword in hand);
-- 10 ticks, the hit at 5: a wide slash on the panel ahead, 16 pixels up,
-- a shake and a flash. Then the next search.
local function slash(me: Object)
    if me.phase_init == 0 then
        me.anim = 5
        me.anim_loaded = 6
        attachment.attach(me, parts.blades.sword, "held", { anim = 0xB, while_dimmed = true })
        battle.play_sound(parts.sounds.swing)
        me.timer = 0xA
        afterimages(me, 5)
        me.phase_init = 4
        return
    end
    if me.timer == 5 then
        local x, y = me.panel_x + me.facing, me.panel_y
        hit(me, x, y, me.element, 0x10_0000, regions.wide)
        local cx, cy = field.center(x, y)
        battle.effect(Vec3.new(cx, cy, 0x10_0000), parts.effects.wide, flip_of(me))
        battle.shake_camera(1, 0xA)
        battle.palette_flash(0, 0xA, true, false)
    end
    if tick_down(me) <= 0 then
        me.phase, me.phase_init = 0, 0
    end
end

-- `sub_80BD586`: the slashes.
local SLASH_PHASES: { [number]: (me: Object) -> () } = { [0] = search, [4] = blink_out, [8] = blink_in, [0xC] = slash }

-- `sub_80BD760`: his sword goes; 3 ticks from now.
local function put_away(me: Object)
    if me.phase_init == 0 then
        me.anim = 4
        sword_off(me)
        me.timer = 3
        me.phase_init = 4
    end
    if tick_down(me) <= 0 then
        me.phase, me.phase_init = 4, 0
    end
end

-- `sub_80BD788`: back beside his user, turned around if he slashed
-- (animation 7) or not (3), on the ground; 3 ticks from now.
local function come_back(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        if s.count ~= 0 then
            me.anim = 7
            me.sprite:set_flip(bit32.bxor(flip_of(me), 1))
        else
            me.anim = 3
        end
        me.pos = Vec3.new(s.home_x, s.home_y, s.home_z)
        me:set_panel_from_coordinates()
        me.pos = Vec3.new(s.home_x, s.home_y, 0)
        me.timer = 3
        me.phase_init = 8
    end
    if tick_down(me) <= 0 then
        me.phase, me.phase_init = 8, 0
    end
end

-- `sub_80BD7D6`: until Colonel is leaving. (Had Colonel's spawn failed,
-- the game would read his action through the null pointer, from the
-- BIOS's open bus, never 8: ProtoMan waits for good.)
local function wait_for_colonel(me: Object)
    local colonel = (me.state :: State).partner
    if colonel and colonel.action == COLONEL_LEAVE then
        me:set_action(LEAVE)
    end
end

-- `sub_80BD742`: back.
local BACK_PHASES: { [number]: (me: Object) -> () } = { [0] = put_away, [4] = come_back, [8] = wait_for_colonel }

-- ---- Colonel ------------------------------------------------------------

-- `sub_80BD834`: he appears; 4 ticks; his charge from its start; to the
-- strike, or he waits (his panel isn't solid).
local function colonel_appear(me: Object)
    if me.phase_init == 0 then
        battle.play_sound(SOUND_APPEAR)
        me.timer = 4
        me.anim = 3
        me.phase_init = 4
        return
    end
    if tick_down(me) > 0 then
        return
    end
    (me.state :: State).count = 0
    if field.solid(me.panel_x, me.panel_y) then
        me:set_action(STRIKE)
    else
        me.phase, me.phase_init = 4, 0
    end
end

-- `sub_80BD878`: no footing: 40 ticks, then he goes.
local function no_footing(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 4
        me.timer = 0x28
        return
    end
    if tick_down(me) <= 0 then
        me:set_action(COLONEL_LEAVE)
    end
end

-- `sub_80BD81A`.
local COLONEL_APPEAR_PHASES: { [number]: (me: Object) -> () } = { [0] = colonel_appear, [4] = no_footing }

-- `sub_80BDA08`: the first panel with an enemy navi along his row ahead;
-- none: his side's fallback column (a column further when he stands
-- there).
local function target(me: Object): (number, number)
    local step = me.facing
    local x, y = me.panel_x + step, me.panel_y
    local enemy = ENEMY_NAVI[me.alliance]
    while field.valid(x, y) do
        if bit32.btest(field.flags(x, y), enemy) then
            return x, y
        end
        x += step
    end
    local column = FALLBACK_COLUMN[me.alliance]
    if column == me.panel_x then
        column += step
    end
    return column, me.panel_y
end

-- `sub_80BD8BC`: once his charge is done, the strike: his sword, an X of
-- two diagonal hits on the target (no element), its two effects, a shake
-- and a flash; 30 ticks.
local function strike(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        if s.count ~= 0xC then
            return
        end
        me.phase_init = 4
        me.anim = 5
        attachment.attach(me, COLONEL_SWORD, "held", { anim = 0x1C, while_dimmed = true })
        local x, y = target(me)
        hit(me, x, y, 0, 0, regions.rising)
        hit(me, x, y, 0, 0, regions.falling)
        local cx, cy = field.center(x, y)
        for _, e in CROSS do
            battle.effect(Vec3.new(cx, cy, 0), e, me.alliance)
        end
        battle.play_sound(SOUND_STRIKE)
        battle.shake_camera(3, 0x23)
        battle.palette_flash(0, 0x28, true, false)
        me.timer = 0x1E
        return
    end
    if tick_down(me) <= 0 then
        me.phase, me.phase_init = 4, 0
    end
end

-- `sub_80BD960`: 20 ticks, then he goes.
local function after_strike(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 4
        me.timer = 0x14
        return
    end
    if tick_down(me) <= 0 then
        me:set_action(COLONEL_LEAVE)
    end
end

-- `sub_80BD89C`.
local STRIKE_PHASES: { [number]: (me: Object) -> () } = { [0] = strike, [4] = after_strike }

-- An effect at his attach point 0.
local function at_hand(me: Object, effect: Effect): Object?
    local p = me.pos
    return battle.effect(Vec3.new(p.x + GLOW_X * 0x10000, p.y, p.z + GLOW_Z * 0x10000), effect)
end

-- `sub_80BDAE0`: his charge, each tick of the strike: the glow (`sub_80BDB04`);
-- once ProtoMan is back and 40 ticks have passed, the glow ends in the
-- charged flash (`sub_80BDB3C`); 15 ticks later it is done
-- (`sub_80BDB8C`).
local function charge(me: Object)
    local s = me.state :: State
    local step = s.count
    if step == 0 then
        local glow = at_hand(me, CHARGING)
        s.glow = glow
        -- (A failed spawn: the game writes the timer through the null
        -- pointer, into the BIOS.)
        if glow then
            glow.timer = 0x7FFF
        end
        battle.play_sound(SOUND_CHARGE)
        me.timer2 = 0
        s.count = 4
    elseif step == 4 then
        me.timer2 += 1
        local protoman = s.partner
        if not (protoman and protoman.action == BACK) or me.timer2 < 0x28 then
            return
        end
        local glow = s.glow
        if glow then
            glow.timer = 1
        end
        local flash = at_hand(me, CHARGED)
        if flash then
            flash.timer = 0x1E
        end
        battle.play_sound(SOUND_CHARGED)
        me.timer2 = 0
        s.count = 8
    elseif step == 8 then
        me.timer2 += 1
        if me.timer2 >= 0xF then
            s.count = 0xC
        end
    elseif step ~= 0xC then
        error(string.format("TwinLdrs Colonel's charge step %#x reads past its table (off_80BDAF4)", step))
    end
end

-- ---- Both ---------------------------------------------------------------

-- `sub_80BD7EA`: his sword goes; 5 ticks (to -1), then out of sight and
-- gone.
local function leave(me: Object)
    if me.phase_init == 0 then
        me.anim = 4
        sword_off(me)
        me.timer = 4
        me.phase_init = 4
    end
    if tick_down(me) >= 0 then
        return
    end
    me.visible = false
    me:set_lifecycle("destroy")
end

-- Run an action's phase from its table.
local function run_phase(me: Object, phases: { [number]: (Object) -> () }, what: string)
    local phase = phases[me.phase]
    if not phase then
        error(string.format("TwinLdrs %s phase %#x reads past its table", what, me.phase))
    end
    phase(me)
end

-- `sub_80BD478`: ProtoMan's actions (`off_80BD4A0`) or Colonel's
-- (`off_80BD4B0`).
local function act(me: Object)
    local action = me.action
    if (me.state :: State).colonel then
        if action == APPEAR then
            run_phase(me, COLONEL_APPEAR_PHASES, "Colonel's appearing")
        elseif action == STRIKE then
            run_phase(me, STRIKE_PHASES, "Colonel's strike")
            charge(me)
        elseif action == COLONEL_LEAVE then
            leave(me)
        else
            error(string.format("TwinLdrs Colonel's action %#x reads past his table (off_80BD4B0)", action))
        end
        return
    end
    if action == APPEAR then
        appear(me)
    elseif action == SLASHES then
        run_phase(me, SLASH_PHASES, "ProtoMan's slashes")
    elseif action == BACK then
        run_phase(me, BACK_PHASES, "ProtoMan's return")
    elseif action == LEAVE then
        leave(me)
    else
        error(string.format("TwinLdrs ProtoMan's action %#x reads past his table (off_80BD4A0)", action))
    end
end

-- `sub_80BD9A2`: one of them for `user`, with the chip's panel, element
-- and damage word, reporting to `controller` if any; it runs while
-- dimmed.
local function spawn(user: Object, spec: NaviChipSpec, r3: number?, colonel: boolean, controller: Object?): Object?
    local o = navi_chips.spawn(twinldrs.kind, user, spec, r3)
    if not o then
        return nil
    end
    o.run_while_dimmed = true
    local s = o.state :: State
    s.colonel = colonel
    s.controller = controller
    return o
end

-- `sub_80BD3AC`: his sprite (animation 0, loaded on the tick's sprite
-- update), beside his user, on the ground, his parts; ProtoMan brings
-- Colonel. Then his first update at once.
local function init(me: Object)
    local s = me.state :: State
    local look = if s.colonel then COLONEL else PROTOMAN
    local sprite = me.sprite
    sprite:load(look.sprite)
    sprite.shadow = "ground"
    me.anim = 0
    me.anim_loaded = 0xFF
    local p = (me.related1 :: Object).pos
    local x, y = p.x + look.dx * me.facing * 0x10000, p.y + look.dy * 0x10000
    me.pos = Vec3.new(x, y, p.z)
    s.home_x, s.home_y, s.home_z = x, y, p.z
    me:set_panel_from_coordinates()
    me.pos = Vec3.new(x, y, 0)
    me.visible = true
    sprite:set_flip(flip_of(me))
    me:add_navi_parts("navi", look.ai_index, 1)
    if not s.colonel then
        local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
        local spec = { panel_x = me.panel_x, panel_y = me.panel_y, element = me.element, damage = damage }
        local colonel = spawn(me.related1 :: Object, spec, nil, true, nil)
        if colonel then
            (colonel.state :: State).partner = me
        end
        s.partner = colonel
    end
    me:set_lifecycle("update")
    act(me)
end

-- `sub_80BD984`: his parts off, the controller (if any) moves on, gone.
local function destroy(me: Object)
    local s = me.state :: State
    local look = if s.colonel then COLONEL else PROTOMAN
    me:remove_navi_parts("navi", look.ai_index)
    if s.controller then
        navi_chip.navi_left(s.controller)
    end
    me:free()
end

twinldrs.kind = define.kind {
    id = "twinldrs/navi",
    pool = "actor",
    state = {
        colonel = "bool",
        controller = "object",
        partner = "object",
        held = "object",
        fresh = "bool",
        glow = "object",
        grey = "u8",
        count = "u8",
        home_x = "i32",
        home_y = "i32",
        home_z = "i32",
    },
    -- `sub_80BD388`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            act(me)
        else
            destroy(me)
        end
        me:update_sprite()
    end,
}

-- The TwinLdrs chip's `navi` hook (`off_802CD5C[20]`): ProtoMan for the
-- user, reporting to `controller`; he brings Colonel.
function twinldrs.summon(user: Object, controller: Object, spec: NaviChipSpec): Object?
    return spawn(user, spec, SPAWNER_ADDRESS, false, controller)
end

return twinldrs
