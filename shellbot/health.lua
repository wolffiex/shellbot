local health = vim.health -- after: https://github.com/neovim/neovim/pull/18720
  or require('health') -- before: v0.8.x

return {
  -- Run with `:checkhealth shellbot`
  check = function()
    local shellbot = vim.env['SHELLBOT']
    if shellbot == nil then
      health.warn('SHELLBOT environment variable is not set')
    elseif vim.fn.executable(shellbot) ~= 1 then
      health.warn('SHELLBOT (' .. vim.inspect(shellbot) .. ') is not executable')
    else
      health.ok('SHELLBOT environment variable is set to an executable')
    end
  end,
}
