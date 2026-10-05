--!strict
-- The ball and chain KnightMan wears (his body overlay, his second related object: his actor record's hook puts it on,
-- 0x0800EE7A), as his attacks handle it: hidden while its ball is thrown or swung (0x080C376E), shown again
-- (0x080C3758), both holding its visibility so that it stops following his, and let go (0x080C3784). His attacks
-- take the object once, as they start (AIAttackVars +0x28, his routines' and his chip's own routine's); nothing
-- changes what he wears meanwhile, and without it each does nothing.

local chain = {}

function chain.hide(me: Object)
    local overlay = me.related2
    if overlay then
        overlay.phase_init = 1
        overlay.visible = false
    end
end

function chain.show(me: Object)
    local overlay = me.related2
    if overlay then
        overlay.phase_init = 1
        overlay.visible = true
    end
end

function chain.let_go(me: Object)
    local overlay = me.related2
    if overlay then
        overlay.phase_init = 0
    end
end

return chain
