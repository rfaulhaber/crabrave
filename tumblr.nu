# this is a convenience nushell script I've written for obtaining oauth tokens and inspecting responses
# from Tumblr's API. Feel free to use it if you find it helpful!
# You will need to create an initial tokens.json file with the following keys:
#   TUMBLR_CONSUMER_KEY: your oauth consumer key
#   TUMBLR_CONSUMER_SECRET: your oauth consumer secret
#   TUMBLR_REDIRECT_URI: the redirect URI for your oauth application

const base_url = "https://api.tumblr.com/v2"

# OAuth2 commands
export module oauth {
    # Retrieve the authorization token URL and get an access token, updating tokens.json
    export def access [] {
      open tokens.json | with-env $in {
        let params = {
            client_id: $env.TUMBLR_CONSUMER_KEY
            response_type: code
            scope: "basic write offline_access"
            redirect_uri: $env.TUMBLR_REDIRECT_URI
            state: (random uuid | str replace --all "-" "")
        }

        let format_params = $params
            | transpose key value
            | each { |r| $"($r.key)=($r.value)" }
            | str join "&"

        print $"www.tumblr.com/oauth2/authorize?($format_params)"

        let access_code = input "Authorization token: "
        let resp = http post --full --allow-errors --content-type application/x-www-form-urlencoded https://api.tumblr.com/v2/oauth2/token {
            grant_type: authorization_code
            client_id: $env.TUMBLR_CONSUMER_KEY
            client_secret: $env.TUMBLR_CONSUMER_SECRET
            redirect_uri: $env.TUMBLR_REDIRECT_URI
            code: $access_code
        }
            | tee { print }

        let access_token = $resp | get body.access_token
        let refresh_token = $resp | get body.refresh_token

        open tokens.json
            | update TUMBLR_ACCESS_TOKEN { $access_token }
            | update TUMBLR_REFRESH_TOKEN { $refresh_token }
            | save -f tokens.json
        }
    }

    export def refresh [] {
      open tokens.json | with-env $in {
        let resp = http post --full --allow-errors --content-type application/x-www-form-urlencoded https://api.tumblr.com/v2/oauth2/token {
            grant_type: refresh_token
            client_id: $env.TUMBLR_CONSUMER_KEY
            client_secret: $env.TUMBLR_CONSUMER_SECRET
            refresh_token: $env.TUMBLR_REFRESH_TOKEN
        }
            | tee { print }

        let access_token = $resp | get body.access_token
        let refresh_token = $resp | get body.refresh_token

        open tokens.json
            | update TUMBLR_ACCESS_TOKEN { $access_token }
            | update TUMBLR_REFRESH_TOKEN { $refresh_token }
            | save -f tokens.json
        }
    }
}

export module http {
  export def get [path] {
    let url = $base_url + $path
    open tokens.json
        | with-env $in {
            http get --full --allow-errors -H [Authorization $"Bearer ($env.TUMBLR_ACCESS_TOKEN)"] $url
        }
  }
}
