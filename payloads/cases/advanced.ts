import { TestCaseCollection } from "./types";
import {
  OPENAI_CHAT_COMPLETIONS_MODEL,
  OPENAI_RESPONSES_MODEL,
  ANTHROPIC_MODEL,
} from "./models";

// Advanced test cases - complex functionality testing
export const advancedCases: TestCaseCollection = {
  multimodalRequest: {
    "openai-chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [
        {
          role: "user",
          content: [
            {
              type: "text",
              text: "What do you see in this image?",
            },
            {
              type: "image_url",
              image_url: {
                url: "https://t3.ftcdn.net/jpg/02/36/99/22/360_F_236992283_sNOxCVQeFLd5pdqaKGh8DRGMZy7P4XKm.jpg",
              },
            },
          ],
        },
      ],
      max_completion_tokens: 300,
    },
    "openai-responses": {
      model: OPENAI_RESPONSES_MODEL,
      input: [
        {
          role: "user",
          content: [
            {
              type: "input_text",
              text: "What do you see in this image?",
            },
            {
              type: "input_image",
              detail: "auto",
              image_url:
                "https://t3.ftcdn.net/jpg/02/36/99/22/360_F_236992283_sNOxCVQeFLd5pdqaKGh8DRGMZy7P4XKm.jpg",
            },
          ],
        },
      ],
      max_output_tokens: 300,
    },
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 300,
      messages: [
        {
          role: "user",
          content: [
            {
              type: "text",
              text: "What do you see in this image?",
            },
            {
              type: "image",
              source: {
                type: "base64",
                media_type: "image/jpeg",
                data: "/9j/4AAQSkZJRgABAQAAAQABAAD/2wCEAAkGBxMUExYUFBQXFxYWGRsZGhkZGBgZIBgjIBYYGBkbGhkhHikhHx4mHhsYIjIiJiosLy8wGSA1OjUuOSkuLywBCgoKDg0OGxAQHDAmISYsMC4uLi40MC4uLi4uNy4uLiwsNy4xLi4sLi4uLi4wLi4wLi4uLC4uLi4uLi4uLi4uLP/AABEIAOAA4QMBIgACEQEDEQH/xAAcAAACAwEBAQEAAAAAAAAAAAAABgQFBwMBAgj/xABLEAACAQMABgYFCQQIBQQDAAABAgMABBEFBhIhMUETUWFxgZEHIjKhsRQjQlJicpLB0TOCsvAkQ1NUc6LC4RY0RJPSFSWD4jVjo//EABsBAAIDAQEBAAAAAAAAAAAAAAADAQIEBQYH/8QAMREAAgIBAwICCQQDAQEAAAAAAAECEQMSITEEUUFxBRMiYYGRobHRMkLB8BRS4fEj/9oADAMBAAIRAxEAPwDcaKKKACiik/TnpJ0daytDJK3SIcMqRu2DjODgccUAOFFJlh6T9FynHykRk/2itH72GKm6361paWZuo1E42kVQjjDFmCj1wDuoAXfStrZNEY7G0OLicbRccYo8kFgeROG38gDzxVH6ItKNa3M2j55CwlxLCzE+s2D0gyTxO446wx51S2ulDe3tzdsjIfUiVX4oAuWXzrrpnRYmCsrGOWI7UUi8UYEEeGQK5+TrVDNofHj+TPLNU68DeKKy3QnpOeICPSMLqw3dPEpeN+0gb1Pn4U3WWvejZcbN7Bv5NIqHybBrdGSkri7Hpp8DLRUWDSEL+xLGw+y6n4Gu+2OsedWJPuiiigAooooAKKKKACiiigAooooAKKKKACiiigAooooAKKKKACiiigDysp9L+g1hePSkY3xlY5xydGOwGx9YEgZ7uqtWrB/S7aMNITKJZVWSzEuyHOyxV2Uhl4EeoDjrqsqp3wVlVbkuSCOQesispHNQcg1UX+rYMbJBI8QYglAxMbEEMpKHhvAORU7QDk20JPHo0/hAqfXnPWzwyai+DDqcXsKur948dzNFMoSSUiRRncxxg7J55xnwNNVV2nNFLPHsn1XXejjip4jwrhq3pNpVaOTdNCdmQdfIMO/H85q2WssfWR8KtfRPyCXtK0XFQrnRMEntxRt2lRnzG+ptFIjOUd06KJ0Ub6o2ZOREVP2XcfnXo1ZhHsyTr3TPV3RTV1OZfufzLa5dypj0TMm+K9ukP+KW+NTbbTOmYTlLxJ1+pNGN/wC+Bn31JopkeuzR8b8yyyzXiSX9MUsAAurBgfrRyDZPcSPdmrLR/pqsHx0sdxCOG0yB1/ErE+6qNlBBBAIPEHeD4UtX+rzREy2Z2W+lEd6P2YO7PZ8K34fSCltNU/p/wdHPfJu+hNYrW6GbeaOTmQrDI714iravzZo63trv10Vre4jO8xnYZG4ZHWMg9opx0B6QrmydYtI/OwHctyo9ZeyVRx78Z7+WuHURlLS9n2f8dxscibp8mx0VxgnV1DowZWAIIOQQeBBrtTxgUUUUAFFFFABRRRQAUUUUAFFFFABRRRQB5WC+mO7Bvrhgf2NnHD+9JKx+De6t1nmVFZ2OFUFiTyAGSa/M17dNdzq7DDXtwZsH6MUe5AfAHypeSSUd/wC+JSbpDdouHZhjX6saDyUZqTRRXmZO5Wc8KWdYswTxXS+yx6KXtB4E92/yFM1Vuslp0ltKmMnZLDvX1h8KZ08lHIr4ez8mWg6ZZZoqp1UveltomJyQNg96nGfEYPjVtVMkHCTi/BkNU6CiiiqEBRS1d61HpWiggecp7RXJHbjAO7tr211yh2tiVJIW+2N36jxFaf8ADzadWkv6uXYZKK5wTK6hkYMp4EHIrpWfgoK+tNi0TLeQ7njPzgH013bz8D2d1XkLx3EIbAaOReB947wfhUmaEOrIRkMCpHYRg0s6l3QSKWJ3A6KVlG0QNx38+3Na7lkxX4xa+T/AzmPkNPow049ld/8ApsrFoZctbsx9k7zsdxwd31vvVs9fmzWnSMQktpo5ELxTKfVYHAyCc45bq3BNedGnH9Mgz/iCu1085Txpy5NeOTcdxkoqmt9arJ/ZuoD/APKn6186b1ptLaLpZp0C8sEMW7FUbye6nDC6oJrJ770i31yCLC16NDwmuN2e1UH+9Uc+gL2433mkZn+xF6i927A/yip0ipZoR5ZuCzKTgMM9WRXWsDm9H0ABaCSWOYb1k2zx7ezup99E+tcl1E9vcE/KbU7EhPFxkhW7T6pB8DzoaoMeWM+B/oooqBoUUUUAFFFcLq4SNGkdgqIpZmO4KAMkk9QAoAQPTLpplt0soiOlu22D9mMb5G+A7iazTVmJZJ5Jl/ZxAQRdyjefH/VXDWLT73ElxpE5+dPye2X6q8CwHXuJ72NMGgrAQwxxcwMt3ne389lc3rstRaXl+TNmkWFFFFcUyhRRRQAq6nfNy3NufoPtL3Zx8NnzpqpSvz0Gk45OCzrsnvwF+ISm2tXVbyU+6T+PDLz5vuFU2tukuht2YHDP6i9mQST4AH3Vc0pafXp76G34qnruP83wAH71HR4vWZknxyRBb7jXqTogW9sgx68gDue0jcO4DA86tr6ximXZlRXHUwz5dVdYvZHdX1XpxLk27Eq81MeBjLYSFW4mFzlW7AT+fmKrLTSWkLmVoI1ihdB64cnaHWQDnPgPGtIqLcaOieRJWQGSP2X4MOWMjiN53HdSZ9PjnLVKKbGLN/tuKq6kyyft72RuyP1R7/0qVD6PbEe0sjnraQ/BcU1UVeMVHZKijyz7lHHqfYj/AKdPHaPxNdP+FLL+7R/hq4oq4esl3KKTU2xbjboO7aHwNQZvR5ZHeokjPWshOPxA0yX17HCheV1RRzY48B1nsFJmkdeXclbWP1f7STd+FP18qgspTq72Lu10BcRn5u+kYD6MqLJ79xq8gDBRtkFuZUEA9wJJHnWRaSubhlZ5LmUkAnCsUUdgUbq1HQAcW0O2SW6NNok5JOyOJ66mqIlUlad/An0vw3PyPTFtMN0d38xJ94kKhPiU8jTBSTr1eq6yIMrLbGOUA7toZB2kPVxB6sVDLYG1NH6DoqJou6EsMcg3iRFbzUGpdLOmFFFFAHlZV6YtOPI0ejIGw03rTsPoRgjA8d58AOe7RtM6SS3glnkOEiUufAcB2k7vGsG0FI0nyjSE/wC0nLSb/ooMkAdmBjuUUjPl9XBtc8LzF5JaUQkgWW+SFR81aIDjltbse8j8JpupY1EjLRSTt7U0jHwB/UmmeuH1crnp7bfHxMeTmuwUUUVmKBRRRQAs6/WZaBZF9qJg2ew7ifA7J8Ku9E3wmhSUfSG/sPBh55rvPCrqyMMqwIPcd1J+q1w1tO9pKdzNlDyJxu8GA8wa2RXrcLj4x3+D5+Qxe1Guw6Uq6rJ0t5dXHIHo18wD7kXzpi0jcdHFI/1EZvJSRVRqDb7NorHjIzMe3fsj4Vr9FQ3lL4FVtFv4DxF7I7hX1XxAfVHdX3XaM4UUUUEBRRXtAHlV2sGlBbQPKV2tnAA6yThcnkM1ZVEuY4Z1khYq4I2XUHeM9eN4PV3UFo1e5lk8klw3S3DbbHeq/RQcgq8K6VN0tq5Pa5KAywciN7p2MvMdoqnbSMeztBgezn3YqyaF5oZJS7rwolW1obieK3HBmDSdiKcnz4eIrWgOrhSrqHoVokaeQYkmwcH6KcVXsJ4nwpqqrdjWlFKK8PuFKfpB0GZoTNHnpYlbOPpoR669uOI8abaKCIycXaL/ANFl4JdF2jA52Ygh7ChKEHyptrHtQr02GkXs23W93mSH7Mg9pB37/Je2tgpbOtGSkrR7RRRUFjKPTppUskFgh9a4cO+PqIcj/MM/uUq6xER2MiruAjCAdQOF+FfWtt10+mrh+It0WJezcc48WfzqNrof6E/7n8QrmdXLVnhDs19TLldzSJurEGxawr9gMe9iWPxqzqLor9hF/hp/CKlVycrucn72Z5chRRRVCAooooAKp9YtBLcJx2ZF9h+rng9nw41cV47AAknAG8k8qvjnKEk48kptO0IWkdPSLbyW1whEwUKG+uM7yT3c+dO2hbfo4IU+qij3ZPvpB1k0kbqVNhT0KSBFbHtMx6+4ZA6t/OtKAxu6t1el6XHphdU3u0Xy7JFlan1RXWuNn7A8a7VpMrCo9xpCGMhXljRjwDOqnyJqn1p0tKjR21sNq5uDsoPqDm5/36ieVUmu+gbWwiS12RcX9wNqWeQk9Gud5RSdxJBAJ34UnqqG6NODpZZarx2XvHpWBGQQQeY358aVX0i9jOEmYvaysejkYkmJjv2XPNeOOod1LttY32j4kulDm3fGVb2WGMggfRyM4O7xp1YQ6QtTjfHKOfFGHWOTKf530J2Mz9LLppaZ8e7f7eKPjXS7litJHi3NlQWG8opYBnHd18s5pF1duBbTxSKTsSMI5DnO0HIwx6yGwfOnbVBZTbGG5Q7UTNEdobpFAGCDzGDjPZVPDqKwuADIPkysHC79vcciM/ZB59VBn4dXxz70PFQn0VAXEhhiMg3hthc9+ccam15Uik6OV3cpGjSSMFRRkk8qr9BaxQXW10THKcQw2TjkQOqk/XnShnm+TIfm4jmTH0n5L+78SeqoWgJhDewsMASZibluYbveBRXiXqN6Xy9/75jnpu5Z7u2tlOASZ5MH6KZ2B3Fx7qg3+tkhuvk1tHHIwOyekfY23/sozw2uW87zuq3m0ds3TXhb2YNgLjhhi5OeqlyLVcPoZb0ZEzyvIzAnJBkIU9hUgHI7aq3Rt6Pp455qL7bebOusFwbu1M0QaK5tJA5Q7njZT6wPxH3a2vVnTC3dtDcLjEiBiByPBh4HIrFZNJZitdKMN8hNnfAcGI3JI3eqjJ7QO9t9ClyYvldgxz8nl20+4/V2ZGf3jUPcZCOhuHyNSr2vKKgbufmzQ8m3cXsh4tcSfxsfzqfrAu3ZzLzVdryOarNX12Z7yM8VuH/jZfyq7kXIKngwKnuIwa4fUvT1DfZpmLJtM91Yn27WFvsbJ71JU/CrSlTUOfZWW3b2omJHcTg+/wCIprrL1MNOSS9/0e4uaqTCiiikFQooqNfXscSGSRgqj39gHM1Ki26QHeRwoLMQAN5J3Ad9KjPNpKQwwZS2U/OSEe1zxjywviaqNIabNzIBKJEtgc7MYyz44ZPDf7vfTjofXLR6qsK7UCjcAyEAd5GfM13Oj6D1ftz5+wynFbK39iNrRo6OI6OgiXCCbPacbBJJ5k78mmGvNM6ME0kEwb9izMAN+0GUAYPZuNdoICx7K6YqUrSJtquFFfUkgUFmOAoJJ6gBknyr6pe1/vOisZetwI/xHB92akXFamkWXob0f8omudJyDe7GKHP0VGNojyUfi6zSZpGf5bpmdicqZlgX7qsEOPInxra9QNGi10fbRYwViDN95vXf/Mxr8+6hXHz8crc51YnvYE/GlS4Z6b0XjvMq8E2vgj9C64WKvYzpjhGSB2qu0uPECsG1M0j8muuhJ+ZuOH2X+j+niK/Qes8oWzuGPKJz/kOK/MumYSY8jcyHaBHL+ePhRdMfg6T/ACeiy3zF2v5NlryoOgtIfKLeKbm6gnv4N7wanU08i1ToK5XcjLG7KMsFYgdZAJA8660UAYzoyQFTk5diWbPHaJ35FfekZNlQ/ON0ceDCtP0rq7bT75I1LfXX1W/EPzqgf0eQkj5+Ux5yUbZOd+cbX+1Te1FlGLnrvxsZNYpMW1w3VE5/ympdlGF1bjB/sVPiXz+dQtZVza3AHOJ/4TS4NcDJouCzCbJREDNncQvs4HWd2e7t3Knwdr0H0882ZaFw035BqDbC4t9KWB3h4lmQdTAcR4hK89HmlyukbGUn1buBoZPvptDPflY/xGpfoYH/ALrN1fJGz/3ocfnSlouXorS0uAcC3vTg/ZJUn4UIZ6QiodTJLuz9SUVH+VrRUGc/P+s9r8l01cRnctz84v7wLfxBxUymr05attLAl7EPnbb2scTHnJPbsn1u7apJ0RpBZ4lcceDDqPMVy+vxO1kXkzNmjvZS6c2redLpAcN6rgd35gea02Wt+GUMPWUjII51EurdZEKOMqwwf566WLaeSxfo5MtAx9Vhy/36x4is+lZ4JfuX1X/Clal7x8WdTzr7DDrFVFvOrqGRgyngRVfpDSbbfQwLtzHl9FBjeznsrPDppTlpjyLUbLTTenIrdfW9Zz7KDi36DtpNn6Sd+lnOfqx8k7Ki6KUuWlc7TE4DHn21ZZrv9J0UMKvl9/wIzZnBuEfi/wABUHSsZfYiUZeRgq+ePzqcas9QdHdNO10w9SH1I+1sbz4A+bDqrbJ7Cumj7Wvt/UaBZW4jjSMcERUH7qhfyrtRRVBgUn+lAZtoxyM6A+TU4VT62aGa6t2iUhXyGUnhkHIz1Z4ZoL42lNNmu2q+ov3R8BX5YsLUwyTwHcYpWXyYr+Va9ov0prAqR6Qt5YHUBTIq9JG2N2QRvGeON9Zzr3f2jX3ym1mSSK5UFwMhkcbjtKQCARsnP3qU0ek9F544upjNvbj57DNpjX6Wa0W3KYYgK75ztAY4DG4nG+kx0yCOsYrwSqeDDzFePOq7ywA76oe2x4+nw42oUk93v3Gj0WTlrRkP9XKwHcVVviWpxpP9F1uwtncjAllLL2gKq588+VOFPPk/UV610FFFFSJCiiigDndQ7aOn1lK+YIrI9GnEYUnemVbsIJBzWw1Q3ep9lJKZXhyzHLes4BPMlQcb6rKNnX9E+lP8DJKdXaquBY1I1i+TvdmGN5riVFiiVBuQAEs7vwAB2fKuusehvk+h+iJBZGRmI4bTPvx548KebKyjiUJEiovUoApG1r0wZtGPIcevOVTHNVkJU+S0VQnL1UuoyubXMr+Ze/8AGx6z+IfrRUP/AIAk+qPf+tFGw+jd3QEEEZB3EHnWCa86nS6Mma5tlLWkhy6j+p35wfs7zsty4Gt/rlKgYEEAgjBBGQewilSipKnwS0mqZ+ftH6QjmXajbPWOa9hFSJYlYFWAYHiCMg03ay+iCB3M1jIbWXjsgExnuUHK+GR2VmulIL+O4Ojy8TykDLxEnYHPaO7ZIAyd3MczXOn0EtX/AM3t9jNLC1uiAujS1x0Vizhj+0Ib1EHAk92/4CtE0Hq7DbwmNRtFwRI59p88cnkN+4V11f0JFaxCNBkne783PWezqHKrOupjhoVcvuZcmVy2XBmektT7m2yYPnouOzuDr4c+8eVVCX652WyjDirjZI862SoV/ouGYYliSTtZQSO48RTU2hctE/1LfujLOjed1gh3u/E8lXmxPVWqaJ0ckEKQx+ygx3nmx7Sd9c9F6Ggt89DEqbXEjJJ7MnfjsqfUN2DajHTHgKKKKCoVSa3ySJb9LGTtROkhA5qp9YHrGCato7lGZkDAsmNpc71yMjI7a6MoIIIyCMEHn1g0Fk6dkbR19HPEskZDIw78dasORHUajXOr1pJve3iJ69gD4VRz6kmNzJZXD27Hiu9kPv8AjmvOm0xDxjhnA5qQpPw+FA3Sv2y/gsG1JsT/AFA8GYfnXW31RskORboSPrZb4mqg643Ef7bR0q9qZYfw499dI/SHbfTjmj+8mfgagms39Y3KoAwBgDgBuxRS5Dr1YN/XY+8rD8qlprVZH/qYvFsfGpFOEuzLiuV1dJGu1I6ouQNpiFG/gMmoS6wWp4XEP/cX9a+LrStk6FHnhZG3FWdCD4ZoI0vxRaIwIBBBB3gjeD3Gvi5nWNGdtyqCx57gMndVLb6e0fDGsaTxqijAUNtY59pqJca+WvCNZZm+rGhOfE0Fljk3si80RfNNEspjaPayVVjv2fok9RPHFfWk9JwwLtzSKg7TvPcOJPdS4LrSlzuSNLRD9J/WfHYuN3kK7Qas20ANxcyGV13mWY5APH1VJxx4DfUFtCT3+S3OMmkLm/yluphtzuadxhnHMIvLv+FQ9DaKXSN7BawD+hWeGkfiGxvO/mWI2R3salQG70u5hs1MNsDiS4YEBh9Uf+IOesgVsWq2rcFjAIYFwOLMfadsYLMev4VDZsw463ar3Ft0S9QorrRVDQFFFFACj6R9bV0faGUb5ZDsQrxyxGdojqUb/Ic6z/U7QbQRtLMS1xMduVjvIzv2c+89prjpy6OkdMuTvgsRsoORcEZPeX2vCMVx1i1xaKXoYEWR1/aMxIVfs7ufX31eKMnUTb9lfEbaKQf+ObocbZG+67D4irbRGvEErrHIjxSMcANvBPIBh19uKsY1Bvdb+QxXd3HGAZGCgsFBPDJ4DPLNd6j39mksbRSLtI4wR/PMHfSRPp6WwjltpiXZUJtpcZ2wfVAbtXd5Y6qC0YauORhtdLPNePHF+ygB6Vse253KgPUN5PdV7VNqhozoLaND7bDbkPMs285PYMDwq5oKzq6QUUUUFBc1l0PKZFurVtmeMbJU8JF+qRwzvP8AOKNB64Qyno5fmJgcNG+7fz2SfgcGmOlrWPR9rPKIpFBkKbYI3HGdnc3luqB0WpKpDNXlIUWh723/AOVuSyDhHLvHgeHlipCa23kX/MWRI+vEc+7ePfUkeqv9Lsdga+XUHiAe8A0qw+kGzO5zJGepoz+WasodbLFuFzH+8SvxAoIeOa8Cwk0bC3tQxnvRf0qI+rlo3G3hP7grsmmrY8LiE/8AyJ+tenTFv/eIf+4n60Ee0u5DOqVif+mj8iPzrwao2P8Adk8j+tdJtZ7NeNzF4MG9wzVXca/2Y3RmSVupEbf4nFQWSyviy3i1dtF9m3iH7gqcqxxqSAqKOJwFA8aUH1jv591vaiJT9OU7/wAO7866R6mvMQ99cSSnj0anZQfz2YoLOP8Au/5PvSmvMYJjtUa4kAJJUHYXG8knifDd21K1A1TXSq/LL24Myq5UW6ZRUI34fG/BGOHI8TV3YWEUK7EUaovUoxnv6/GqLVK8/wDTNLdEd1vf4A6lfaOyPBmK9zjqoY/p5Q1UkbTZWkcSLHGioijCqoAA7hUmiilmwKKKKACqzWPSHQWtxP8A2UTvjrKoSB4kAVZ0nel2bZ0TdfaQL5uooAyvU12g0bNcn1pHMkhPWVJUHzBNKujY8JtE5Z/WZuJJPHfWk6owgWMCkZBiGQee1knPmaUNP6qy2xMluDJDxMe8tH93mV99OTo5WW8mqKe9/P3FdX3oa16a9hj4iM9K3YFwR79nzqCukoyhYNwHDn3U6+jrRRSJriQYefeOxAfV8+PdiiTE4YODcpKq2+I4Uka+KJbqxgwMtIWY88bSAjuwGPgKdqSbo7emowf6uHI/C3/lUDsXN9kx3qq07p+G1Xalb1j7KLgs3cOrtO6vjWrTXyWAygbTkhEHWxBIz2AAnwrMkgd3MszGSRt5J5fl4cBQlZX2YrVL/wBLHSeuV9JmSPEMa7woAJI+0SN/hitK0Xd9LDFL/aIreYBrKb72G+6fhWkanf8AJW/+EPzqWqDXrhdVuW9KOuTGK6spuRZom7m2cfEnwpupS9Ji/wBHjfmkyEe8VUti/Ui3orwHO+igDnNbI/tIrfeUH4ioEurto3GCPwXHwqzooBSa4KVtUrM/1I8C3614NUbP+xH4m/Wruign1ku5Vx6tWi8IE8Rn41Pgto03Iir91QPgK60UEOTfJKsY8naPKptcbQeqO3fXapFsKWvSDo4y2pkXdJAelUjiMe1jwGf3RVhrNpxLWEyEbTk7MafWb9BxNUOp2sU1xJJBchSWQsuFC7uDqccRgjt40Dcaa9pGyamaaF3ZQXA4yINrsYEq4/EDV3WX+g25KR3Vkx328xKj7L5x71NajSmdNO0FFFFBIUlemGMnRVzjkFPk6060v6+2Zl0ddxgZLQSYHWQhZR5gUAZ1qdKGsrcjlGoPh6p+FXNJXo5vv6Ko47LMpH7xYe405owIyKacnIqkyovtWLSVw7xLtg5JGV2vvAbm8auAKKKCjbfIUixt/wC9v/hgf5Up6pDvRs6Z+9EP4f8A60DcXj5DDrnoo3Fs6L+0QiRPvLnd4gkVndlciRA3Pgew8612J8gGs71z0N8nl+URj5mQ/OAfQY/S7m+OeupToXKOuOnxXH4KPSRxG/3TWo6tx7FrAvVEn8IrLNJjaj2RxdlUduTWwwR7Kqv1QB5DFEuSuPbF8fwfdKHpPb+iqPrTIPiab6SfSQ+09nCOLzbXgCi/6j5VA3D+tF9GNw7h8K9r015UAFFFe0EHlFQtIaXgh/ayop6s5P4RvqBYXt5fNsWFsxXnPKNlF7RyPdvPZQXjjlLhFjpLSUUKbcrhRy6z2AcSai6IvJpmMjIIoceorD13+0fqjsqovNX5LTSccV24uGeLbV2B2do8lB+qVI8QcCmygtOOjbxLS39le6ugrjaH1RXWpM5lem7/AOVXTyZzHESkQ5bj6zeJ/LqrpoF9m+tj9Yuh8UzVbb27W7Nbyeq6McZ3bQ5MvWDU/RAze2g59IT4BDmrftJd+urwp15UP+pLGLTsy/RuLUPj7SlB/pfzrYKxy3bY01YMPppKh/CTWyUmXJ08LuCCiiioGhXw6ggg8Dur7ooA/OWrsHya9u7Jt2w5ZO4EY/yMh8Ka0cjgcVz9M+hnguIdKQrkAiOcDuwrHsK+rnsWudpdJKiyIcqwyD+vaKujB1EKlZYJenmM10+WjqPuqjttMQSSGNJUZxxUHq44PA+Fdby/iiAMkioDuG0cZ7qkRp8KLY3vUKSdZpiukbSU7g/qHz2f9YpjuLtEjMjMAgGS3LH50n61aRjng2l2kkhZJFEilCwY4BUHiDuPhQNxR3+hoVjJ9HxH513ubdZEZHUMrDDA8CKo9E34mijmU+0M9x4MPA5FXsMoYZ86BElTEyx1FaO5RjKGt4220Q52sg5VTuwQDjfnlwp3ryipCUm+QrONZTNcaRK25UNbIMFuAPFuR37wOHKnvTOklggkmfggzjrPBQO84FZ1qjpaKIzNcMY5ZT0u0wIDKcuNk45kkjryMVA/CnTkkWkc2lgN8EUmOakD/UPhX38t0p/cl/H/AL1bR6XuVhS6a02bNnVelZwHwzbIk6PkmcDfv3g1Omv7iSSWOztxOYBmVmcIqnGejU/ScjwGaLLaZ3WlC50WmH4RwRDrJB/NvhULTWibiGEy3l8QvARxDBY8lBwvw3CmjQ18dJTrBBMYYxGskzjBkG1uESA7gQc7TYOKk6q6lJLpG4W8la4NkyCONwMMrqHjkfkd27HMrv4YqLG48c3u6X3OHoh9HKlPll7EGL4MMbjOBx6RlPM8s8Bv51ssUYUBQAAOAAwB3CulFVbNRmHpy0Iz28d5EPnbVton7B4nwYKe7aqh0berNEkq8HGe48x4GtnuIVdWVwGVgVIPAgjBBr8/GzbRV69lKT0Ep24XPUdwyeGd2ye0A86lMz9Rj1K0N9g/EeP61LqqjfBz1VaRuCMirHPZD0nomGddmWNXA4E5BHcw3jzqForVa2gk6SNXL4IBdmOyDxxmrlWB4EHluOapbrWaKObomD7KsqPKFJjjdvZRm5E+6gtHW9kfV1/+U0Yf/wBsg/8A51s1Yho3RMlxG2loZCbiCR2hhyDGY4iUZCOO24DHOeYrYtFaQSeGOaM5SRA69xGaozo4YuMEmTaKKKgaFFFFAEe8tElRo5FDo4KsrDIYHiCKxjWn0V3FusjWMskkDEF7bJ28bQJEbczjuJ4b62+vKmyGrMF1m0how2cD2iJDcWs0fzOyI5iCdmRGU+s56zv99e6k/I3uZX0pFsSyHZhS7QogTA3JtgLt5/241smkNXbSZ1llt4nkQgq7IpIIORvxnjUrSWjIZ0KTRpIh+i6hh76LI0q7Pz9Bo+dZTEtvJcWdjdHbaMiQsgO1GuxxbZBBOM00axXuj7+/0YcLK228UsLIytslMoXQgYCsDuO7fWnaA1etrNDHbRCNGYuQCxySAM5JJ4AbuFWPQLtbWyu114GfOiyVFLg/P2lbGXQ1w8Uis1nKxaKQAnZzyP2hwI54yOdWGir27umlNhAsscXtSO+yHOM7CDmcVt17ZxyoY5UWRG4qwDA+BrL5Uk0FNO0Vu8tlcesixjaaGXGyqsCc7DbgDnqHHcZsW8MG7ZU2+szzCFLaEyXE2QIicCPZOJDI2NwU5HLNfF5p2cA2/RCO+MywdGW2lUsNoSg802d9StFap3mjDDpGONp3dGN3CuNtNthIehGN+N4K54jnndOsNCQ6Zu7m82ZoVjEMVvL60ciugZpG2TuOCwXfnhyosqungVGkdRro3lpaXN309tKzSbZXYLFELNHgE5yBkb+BY8q1XTOq1ncxiOeCNwq7KnZAZBjACMN643cOoVRRaqXxltXnvY5UtpTIPmOjdsoyEFg+zwY/RFPFRY5RS4Mr09qjpNLOSxgaO5t5FCIZG6OWEBgVyd6uBjGdxxira31MuLF3fRssfRu221vOGI2sbJKzA7QyANxB4U/0VBJm+q/o1hFuPlcKpc9JI/SQOysoZyygSLgnAxxpi1b1QS0mlmE88zyqiEzOHwEzsgHAJ48yaZqKACiiigApY181Pi0jAYn9WRd8cmMlD+angRTPRQB+dDfXWj26C/hk2V3LMoLKwHA7XBhjG/OesV9aQ1tidFjt51V5mWPbb1REG3NIxPDA+NfoSWJWGCAR1EAjyqvn1etHBV7eFg3EGNDnv3VbUJeCDdmUay6HTRcaXdiduHCxzRGQEMx3RzBt+CWwDy4V20UDDo+SyvNHXZM220skUPTBnckiRXQnePVIz1U9n0d6M/ucXHOBtBT3qDg+IpoVQBgbgKixiik7M01A1KtpbGFri0aOX1lfPSxF9liquyZG9lAO/r6q0LRej44IlhiUJGgwqjJwPGplFQWCiiigD//Z",
              },
            },
          ],
        },
      ],
    },
  },

  complexReasoningRequest: {
    "openai-responses": {
      model: OPENAI_RESPONSES_MODEL,
      reasoning: { effort: "high", summary: "detailed" },
      input: [
        {
          role: "user",
          content:
            "There is a digital clock, with minutes and hours in the form of 00:00. The clock shows all times from 00:00 to 23:59 and repeating. Imagine you had a list of all these times. Which digit(s) is the most common and which is the rarest? Can you find their percentage?",
        },
      ],
      max_output_tokens: 20_000,
    },

    "openai-chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [
        {
          role: "user",
          content:
            "There is a digital clock, with minutes and hours in the form of 00:00. The clock shows all times from 00:00 to 23:59 and repeating. Imagine you had a list of all these times. Which digit(s) is the most common and which is the rarest? Can you find their percentage?",
        },
      ],
      max_completion_tokens: 20_000,
    },

    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 20_000,
      messages: [
        {
          role: "user",
          content:
            "There is a digital clock, with minutes and hours in the form of 00:00. The clock shows all times from 00:00 to 23:59 and repeating. Imagine you had a list of all these times. Which digit(s) is the most common and which is the rarest? Can you find their percentage?",
        },
      ],
    },
  },

  reasoningWithOutput: {
    "openai-responses": {
      model: OPENAI_RESPONSES_MODEL,
      reasoning: { effort: "low" },
      input: [
        {
          role: "user",
          content: "What color is the sky?"
        }
      ],
    },
    "openai-chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [
        {
          role: "user",
          content: "What color is the sky?"
        }
      ],
    },
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 20000,
      messages: [
        {
          role: "user",
          content: "What color is the sky?"
        }
      ],
    },
  },

  toolCallRequest: {
    "openai-chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [
        {
          role: "user",
          content: "What's the weather like in San Francisco?"
        }
      ],
      tools: [
        {
          type: "function",
          function: {
            name: "get_weather",
            description: "Get the current weather for a location",
            parameters: {
              type: "object",
              properties: {
                location: {
                  type: "string",
                  description: "The city and state, e.g. San Francisco, CA"
                }
              },
              required: ["location"]
            }
          }
        }
      ],
      tool_choice: "auto"
    },
    "openai-responses": {
      model: OPENAI_RESPONSES_MODEL,
      input: [
        {
          role: "user",
          content: "What's the weather like in San Francisco?"
        }
      ],
      tools: [
        {
          type: "function",
          function: {
            name: "get_weather",
            description: "Get the current weather for a location",
            parameters: {
              type: "object",
              properties: {
                location: {
                  type: "string",
                  description: "The city and state, e.g. San Francisco, CA"
                }
              },
              required: ["location"]
            }
          }
        }
      ],
      tool_choice: "auto"
    },
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 20000,
      messages: [
        {
          role: "user",
          content: "What's the weather like in San Francisco?"
        }
      ],
      tools: [
        {
          name: "get_weather",
          description: "Get the current weather for a location",
          input_schema: {
            type: "object",
            properties: {
              location: {
                type: "string",
                description: "The city and state, e.g. San Francisco, CA"
              }
            },
            required: ["location"]
          }
        }
      ],
      tool_choice: {
        type: "auto"
      }
    },
  },
};
