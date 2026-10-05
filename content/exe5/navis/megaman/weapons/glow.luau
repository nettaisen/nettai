--!strict
-- A glow on MegaMan (EXE5's effect object 0x26, 0x080E330C: EXE6's `sub_80E3150`, the same code): a sprite (its
-- parameters', `sub_801BE2A`: the sprite, its animation, a shadow or none) on its owner for 20 ticks; then it goes.
-- NapalmSoul's charged Fire chips (14-1D) and MagnetSoul's B+Back (14-1B) show one (`sub_80E31C0`'s spawn).

local glow = {}

export type State = {
    sprite: SpriteAsset,
    anim: number,
    shadow: boolean,
}

glow.kind = define.kind {
    id = "megaman/glow",
    pool = "effect",
    state = { sprite = "sprite", anim = "u8", shadow = "bool" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- 0x080E3330: its sprite (`sub_801BE2A`), shown.
            local s = me.state :: State
            local sprite = me.sprite
            sprite:load(s.sprite)
            sprite.shadow = if s.shadow then "ground" else "with_sprite"
            me.anim, me.anim_loaded = s.anim, 0xFF
            sprite:set_flip(bit32.bxor(me.alliance, me.flip))
            me.visible = true
            me:set_lifecycle("update")
        end
        if me.lifecycle == "update" then
            -- 0x080E334A: 20 ticks; it keeps to its owner.
            if me.phase_init == 0 then
                me.timer = 20
                me.phase_init = 4
            end
            local t = me.timer - 1
            me.timer = bit32.band(t, 0xFFFF)
            if t <= 0 then
                me.visible = false
                me:set_lifecycle("destroy")
            end
            me.pos = (me.related1 :: Object).pos
        elseif me.lifecycle == "destroy" then
            me:free()
            return
        end
        me:update_sprite()
    end,
}

-- `sub_80E31C0`'s counterpart (0x080E337C): a glow of `sprite` (animation 0, no shadow) on `owner`, of its side and
-- facing.
function glow.spawn(owner: Object, sprite: SpriteAsset): Object?
    local o = battle.spawn(glow.kind)
    if o then
        local s = o.state :: State
        s.sprite, s.anim, s.shadow = sprite, 0, false
        o.related1 = owner
        o.alliance, o.flip = owner.alliance, owner.flip
    end
    return o
end

return glow
