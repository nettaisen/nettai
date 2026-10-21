--!strict
-- The party supports' actor (EXE5 DS's actor object 0x30, 0x021FF280; docs/design/exe5ds-map.md §4.7): a benched team
-- navi who comes onto a panel the support picked, attacks and goes. Each navi's support makes its own kind of it
-- (`party_support.make`: its navi, its actions) and sends it (`party_support.send`); the DS's one kind with seven
-- variants (its table 0x022270A0: the navi's sprite and NameID) is a kind per navi here.
--
-- The steps they share: the lead-in (4 ticks; the DS lights the navi's button on the bottom screen, the local side's),
-- the appearance (animation 3, the appearing sound), the navi's attack (its own), the warp out (animation 4, 5 ticks,
-- then hidden, its panel and parts given up) and the lead-out (4 ticks; the button goes dark), then it is destroyed:
-- its member no longer busy. It doesn't run while the battle is dimmed; every tick it is shown (to a viewer whose navi
-- isn't blind to the other side's).

local party = require("../rules/party")

local party_support = {}

-- Its actions: the lead-in, the appearance, the attack, the warp out, the lead-out.
party_support.LEAD_IN, party_support.APPEAR, party_support.ATTACK, party_support.WARP, party_support.LEAD_OUT =
    0x0, 0x4, 0x8, 0xC, 0x10

local LEAD_TICKS, WARP_TICKS = 4, 5
local ANIM_IDLE, ANIM_APPEAR, ANIM_WARP = 0, 3, 4
local SOUND_APPEAR = asset.sound("appear")

export type State = {
    -- The member it is (its party block busy flag, +0x68, cleared as it goes).
    member: number,
}

export type Spec = {
    -- Its kind's key.
    id: string,
    navi: NaviDef,
    -- Its sprite (the variant's; else the navi's).
    sprite: SpriteAsset?,
    -- Its palette as it comes (0x021FF3E4: MegaMan's by his block; the others' 0).
    palette: ((me: Object) -> number)?,
    -- What it puts on as it comes (0x021FF434: MegaMan's Hub Style shade); else its identity's parts, a player's.
    wears: ((me: Object) -> ())?,
    -- Its warp out keeps its parts on (MegaMan's, 0x021FF0B5: they come off as it is destroyed).
    warp_keeps_parts: boolean?,
    -- What its lead-in sets as it starts (ShadowMan's 0x021FEC85: no panel aimed at).
    started: ((me: Object) -> ())?,
    -- The appearance's ticks and animation (3 if none said), and what it sets as it has appeared (NapalmMan his shots).
    appear_ticks: number,
    appear_anim: number?,
    -- What it does as it appears, after the sound (KnightMan's 0x021FF18A: a pixel down the field and up).
    appearing: ((me: Object) -> ())?,
    appeared: ((me: Object) -> ())?,
    -- What its lead-out does as it ends (NapalmMan's timer starts again).
    gone: ((me: Object) -> ())?,
    -- What its destruction clears besides its member's busy flag (its +0x6C: KnightMan's guard's slot in the party
    -- block).
    released: ((me: Object) -> ())?,
    -- Its attack (action 8 on), by action.
    actions: { [number]: (me: Object) -> () },
    -- Its own state beside the shared.
    state: StateSchema?,
}

export type Support = {
    kind: Kind,
    navi: NaviDef,
}

-- A count down of the object's timer, entered with `ticks` (its PhaseInitialized byte; the step counts this same tick):
-- true once it reached 0.
function party_support.countdown(me: Object, ticks: number): boolean
    if me.phase_init == 0 then
        me.timer = ticks
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    return t <= 0
end

-- The next action, from its first phase.
function party_support.go(me: Object, action: number)
    me.action, me.phase, me.phase_init = action, 0, 0
end

-- 0x021FE605 and its kin: the lead-in, 4 ticks (the bottom screen's button lit on the local side: presentation).
local function lead_in(me: Object, spec: Spec)
    if me.phase_init == 0 and spec.started then
        spec.started(me)
    end
    if party_support.countdown(me, LEAD_TICKS) then
        party_support.go(me, party_support.APPEAR)
    end
end

-- 0x021FE639 and its kin: it appears (animation 3, the appearing sound) for its ticks.
local function appear(me: Object, spec: Spec)
    if me.phase_init == 0 then
        me.visible = true
        me.anim = spec.appear_anim or ANIM_APPEAR
        battle.play_sound(SOUND_APPEAR)
        if spec.appearing then
            spec.appearing(me)
        end
    end
    if party_support.countdown(me, spec.appear_ticks) then
        if spec.appeared then
            spec.appeared(me)
        end
        party_support.go(me, party_support.ATTACK)
    end
end

-- 0x021FE6F1 and its kin: the warp out, 5 ticks (its first phase its init); then hidden, its panel and its parts given
-- up. What it held (its first related slot, +0x4C) goes as the slot is cleared (the DS clears it before it passes it on
-- to be ended, which then ends none).
local function warp(me: Object, spec: Spec)
    if me.phase == 0 then
        me.related1 = nil
        me.anim, me.anim_loaded = ANIM_WARP, 0
        me.timer = WARP_TICKS
        me.phase = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t > 0 then
        return
    end
    me.visible = false
    me:unreserve_panel(me.panel_x, me.panel_y)
    if not spec.warp_keeps_parts then
        me:remove_parts(me.identity :: Identity)
    end
    party_support.go(me, party_support.LEAD_OUT)
end

-- 0x021FE73D and its kin: the lead-out, 4 ticks (the button goes dark on the local side); then destroyed.
local function lead_out(me: Object, spec: Spec)
    if party_support.countdown(me, LEAD_TICKS) then
        if spec.gone then
            spec.gone(me)
        end
        me:set_lifecycle("destroy")
    end
end

-- A navi's support actor.
function party_support.make(spec: Spec): Support
    local navi = spec.navi
    local state: StateSchema = { member = "u8" }
    for k, v in spec.state or {} do
        state[k] = v
    end

    -- 0x021FE4F9: its navi's sprite, a ground shadow, animation 0, on its panel on the ground, facing its flip's way,
    -- its palette; its NameID's parts (a player's), or what MegaMan wears.
    local function init(me: Object)
        local sprite = me.sprite
        sprite:load(spec.sprite or navi.sprite)
        sprite.shadow = "ground"
        me.anim, me.anim_loaded = ANIM_IDLE, 0
        sprite:set_animation(0)
        me:set_coordinates_from_panel()
        local p = me.pos
        me.pos = Vec3.new(p.x, p.y, 0)
        sprite:set_flip(bit32.bxor(me.alliance, me.flip))
        sprite.palette = if spec.palette then spec.palette(me) else 0
        me.identity = navi.identity
        if spec.wears then
            spec.wears(me)
        elseif navi.identity.actor_type == "player" then
            me:add_parts(navi.identity, 0)
        end
        me:set_lifecycle("update")
    end

    -- 0x021FE58D: shown (to a viewer not blind to it); outside the dimming, its action and its sprite.
    local function update(me: Object)
        me.visible = true
        me:hide_from_blind()
        if battle.dimmed() then
            return
        end
        local a = me.action
        if a == party_support.LEAD_IN then
            lead_in(me, spec)
        elseif a == party_support.APPEAR then
            appear(me, spec)
        elseif a == party_support.WARP then
            warp(me, spec)
        elseif a == party_support.LEAD_OUT then
            lead_out(me, spec)
        else
            local f = spec.actions[a]
            if not f then
                error(string.format("%s: action %#x reads past its table", spec.id, a))
            end
            f(me)
        end
        me:update_sprite()
    end

    -- 0x021FE5C5: its member no longer busy, its panel and parts given up, freed.
    local function finish(me: Object)
        party_support.release(me)
        if spec.released then
            spec.released(me)
        end
        me:unreserve_panel(me.panel_x, me.panel_y)
        me:remove_parts(me.identity :: Identity)
        me:free()
    end

    local kind = new.kind {
        id = spec.id,
        pool = "actor",
        state = state,
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me)
            elseif l == "update" then
                update(me)
            else
                finish(me)
            end
        end,
    }
    return { kind = kind, navi = navi }
end

-- Its member no longer busy (its +0x68's byte cleared).
function party_support.release(me: Object)
    party.supports(me.alliance).busy[(me.state :: State).member + 1] = false
end

-- 0x021FF280: `support` for the side's navi `navi` on (x, y), its flip `flip`, with the damage word, `member` busy
-- while it acts; its panel reserved. Nil when the actor pool is full.
function party_support.send(support: Support, navi: Object, x: number, y: number, flip: number, damage: number, member: number): Object?
    local o = battle.spawn(support.kind)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.alliance = navi.alliance
    o.flip = flip
    o:set_damage_word(damage)
    o.related1 = navi;
    (o.state :: State).member = member
    party.supports(navi.alliance).busy[member + 1] = true
    o:reserve_panel(x, y)
    return o
end

-- A support's dimming controller (EXE5 DS's effect objects 0x91, 0x94 and 0x95: Mag-Barr's, Serch-S's and DiceBons'):
-- the dim, the telop (its chip the spawn names), the navi brought (`bring`, from the controller: its actor's `done`
-- clears the controller's word for it), the undim once it is gone.
export type Controller = {
    kind: Kind,
    -- The controller for the side's navi `navi`, on its panel, with the damage word; its dimming started (the other
    -- side can't cut in), its telop `chip`. Its state is the caller's to fill (`ControllerState` and its own).
    send: (navi: Object, damage: number, chip: Chip) -> Object?,
    -- What an actor it brought calls as it goes.
    done: (controller: Object?) -> (),
}

type ControllerState = { out: boolean }

function party_support.controller(spec: { id: string, state: StateSchema?, bring: (me: Object) -> Object? }): Controller
    local state: StateSchema = { out = "bool" }
    for k, v in spec.state or {} do
        state[k] = v
    end

    -- Its third action: the navi comes (the controller's word for it set); once it is gone, the undim.
    local function bring(me: Object)
        local s = me.state :: ControllerState
        if me.phase_init == 0 then
            if spec.bring(me) then
                s.out = true
            end
            me.phase_init = 4
        end
        if not s.out then
            me:set_action(0xC)
        end
    end

    local kind = new.kind {
        id = spec.id,
        pool = "effect",
        state = state,
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                dimming.begin(me)
            elseif l == "update" then
                local a = me.action
                if a == 0 then
                    dimming.dim_screen(me)
                elseif a == 4 then
                    dimming.show_telop(me)
                elseif a == 8 then
                    bring(me)
                elseif a == 0xC then
                    dimming.undim_screen(me)
                else
                    error(string.format("%s: action %#x reads past its table", spec.id, a))
                end
            else
                dimming.finish(me)
            end
        end,
    }

    local function send(navi: Object, damage: number, chip: Chip): Object?
        local o = battle.spawn(kind)
        if o then
            o.panel_x, o.panel_y = navi.panel_x, navi.panel_y
            o.element = 0
            o.related1 = navi
            o.alliance, o.flip = navi.alliance, navi.flip
            o:set_damage_word(damage)
        end
        dimming.start(navi.alliance, true, o, navi, { chip = chip })
        return o
    end

    local function done(controller: Object?)
        if controller then
            (controller.state :: ControllerState).out = false
        end
    end

    return { kind = kind, send = send, done = done }
end

return party_support
