-- Takes drivers who stopped reporting off the map.
-- KEYS[1] = geo key, KEYS[2] = seen key
-- ARGV[1] = cutoff in ms. Anyone last seen before it is gone

-- Find and remove in one step. Done as two round trips, a driver could report in
-- between them and still be removed.
-- "(" makes the bound exclusive: a driver seen exactly at the cutoff stays
local stale = redis.call('ZRANGE', KEYS[2], '-inf', '(' .. ARGV[1], 'BYSCORE')
-- One ZREM per driver instead of ZREM key unpack(stale): unpack fails once
-- the list outgrows Lua's stack, which a big fleet could reach
for _, driver in ipairs(stale) do
  redis.call('ZREM', KEYS[1], driver)
  redis.call('ZREM', KEYS[2], driver)
end
return stale
