--!strict
-- What the navi chips' navis share (docs/engine/chips.md §3.6): how a chip
-- places its navi, how he stands on his panel, the footing he needs, the
-- timers of his actions, and his leaving. The navi chip controller (the
-- ruleset's) calls a chip's `navi` hook, and the navi reports back with
-- `navi_chips.leave`.

local navi_chips = {}

-- The panel flags a navi needs to stay where he appeared (as one word).
local CAN_STAND = 0x10010

-- A navi of `kind` for `user`, on the panel the controller picked, with
-- the chip's element and damage word (each navi's spawner, `sub_80BC0DA`
-- and kin). His position is what the spawner's registers held: the row,
-- the element and `r3` (the address the controller called him through,
-- or nil for none), of which his init keeps Z's fraction.
function navi_chips.spawn(kind: Kind, user: Object, spec: NaviChipSpec, r3: number?): Object?
    local o = battle.spawn(kind, Vec3.new(spec.panel_y, spec.element, r3 or 0))
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = spec.panel_x, spec.panel_y
    o.element = spec.element
    o.related1 = user
    o.alliance, o.flip = user.alliance, user.flip
    o:set_damage_word(spec.damage)
    return o
end

-- His init: on his panel's center (Z's whole part cleared; all of Z with
-- `ground`), `sprite` loaded at animation 0 and palette 0 with `shadow`,
-- facing his side.
function navi_chips.stand(me: Object, sprite: SpriteAsset, shadow: Shadow, ground: boolean?)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, if ground then 0 else bit32.band(p.z, 0xFFFF))
    local s = me.sprite
    s:load(sprite)
    s.shadow = shadow
    me.anim = 0
    me.anim_loaded = 0
    s:set_animation(0)
    s.palette = 0
    s:set_flip(bit32.bxor(me.alliance, me.flip))
end

-- Whether his panel holds him (a field panel no one else stands on).
function navi_chips.has_footing(me: Object): boolean
    return bit32.band(field.flags(me.panel_x, me.panel_y), CAN_STAND) == CAN_STAND
end

-- An action's timer: on phase 0, phase 4 with `ticks` (and Timer2 cleared
-- when `word`: the original's word store of both), returning nil; after,
-- one tick down, returning what is left.
function navi_chips.countdown(me: Object, ticks: number, word: boolean?): number?
    if me.phase == 0 then
        me.phase = 4
        me.timer = ticks
        if word then
            me.timer2 = 0
        end
        return nil
    end
    local left = me.timer - 1
    me.timer = left
    return left
end

-- The same timer, as most actions read it: true once it has run out.
function navi_chips.wait(me: Object, ticks: number, word: boolean?): boolean
    local left = navi_chips.countdown(me, ticks, word)
    return left ~= nil and left <= 0
end

-- He goes: the controller (if it waits for him) moves on.
function navi_chips.leave(me: Object, controller: Object?)
    if controller then
        navi_chip.navi_left(controller)
    end
    me:set_lifecycle("destroy")
end

return navi_chips
