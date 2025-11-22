def main [access_code: string] {
  http post --full --allow-errors --content-type application/x-www-form-urlencoded https://api.tumblr.com/v2/oauth2/token {
        grant_type: authorization_code
        client_id: $env.TUMBLR_CONSUMER_KEY
        client_secret: $env.TUMBLR_CONSUMER_SECRET
        redirect_uri: $env.TUMBLR_REDIRECT_URI
        code: $access_code
    }
}
